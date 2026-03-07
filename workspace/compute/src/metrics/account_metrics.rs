//! Per-account metrics computation.
//!
//! Computes universal and kind-specific financial metrics for individual
//! accounts using existing balance DataFrames and recurring transaction data.

use chrono::{Datelike, Months, NaiveDate};
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::str::FromStr;
use tracing::{debug, instrument, trace, warn};

use common::metrics::{
    AccountKindMetricsDto, AccountMetricsDto, DebtMetricsDto, InvestmentMetricsDto,
    OperatingMetricsDto, ReserveMetricsDto,
};
use model::entities::account::{self, AccountKind};
use model::entities::recurring_transaction;

use crate::account::AccountStateCalculator;
use crate::account_stats;
use crate::error::{ComputeError, Result};
use crate::metrics::filter_active_recurring;

/// Computes all per-account metrics for a single account.
///
/// This includes universal metrics (balance, funding ratio, flow stats)
/// and kind-specific metrics dispatched by `AccountKind`.
#[instrument(skip(calculator, db, account))]
pub async fn compute_account_metrics(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    account: &account::Model,
    today: NaiveDate,
) -> Result<AccountMetricsDto> {
    let accounts = vec![account.clone()];
    debug!(
        account_id = account.id,
        account_name = %account.name,
        "Computing metrics for account"
    );

    // --- Current balance ---
    let current_balance = get_current_balance(calculator, db, &accounts, today).await?;
    trace!(account_id = account.id, %current_balance, "Current balance computed");

    // --- Target balance ---
    let target_balance = account.target_amount;

    // --- Funding ratio ---
    let funding_ratio = target_balance
        .filter(|t| !t.is_zero())
        .map(|t| current_balance / t);

    // --- Monthly net flows for last N months ---
    let net_flows = compute_monthly_net_flows(calculator, db, &accounts, today, 6).await?;
    trace!(account_id = account.id, flow_count = net_flows.len(), "Monthly net flows computed");

    let monthly_net_flow = net_flows.last().copied();

    let three_month_avg_net_flow = if net_flows.len() >= 3 {
        let last_three: Vec<&Decimal> = net_flows.iter().rev().take(3).collect();
        let sum: Decimal = last_three.iter().copied().sum();
        Some(sum / Decimal::from(3))
    } else if !net_flows.is_empty() {
        let sum: Decimal = net_flows.iter().sum();
        Some(sum / Decimal::from(net_flows.len() as i64))
    } else {
        None
    };

    let flow_volatility = compute_stddev(&net_flows);

    // --- Kind-specific metrics ---
    let kind_metrics = compute_kind_metrics(
        calculator,
        db,
        account,
        &accounts,
        today,
        current_balance,
    )
        .await?;

    let kind_str = match account.account_kind {
        AccountKind::RealAccount => "RealAccount",
        AccountKind::Savings => "Savings",
        AccountKind::Investment => "Investment",
        AccountKind::Debt => "Debt",
        AccountKind::Other => "Other",
        AccountKind::Goal => "Goal",
    };

    Ok(AccountMetricsDto {
        account_id: account.id,
        account_kind: kind_str.to_string(),
        current_balance,
        target_balance,
        funding_ratio,
        monthly_net_flow,
        three_month_avg_net_flow,
        flow_volatility,
        kind_metrics,
    })
}

/// Retrieves the current balance for the first account in the slice at the given date.
async fn get_current_balance(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    today: NaiveDate,
) -> Result<Decimal> {
    let stats = account_stats::state_at_date(calculator, db, accounts, today).await?;
    Ok(stats
        .first()
        .and_then(|s| s.end_of_period_state)
        .unwrap_or(Decimal::ZERO))
}

/// Computes monthly net flows (end_of_month - start_of_month) for the last `months` months.
///
/// Returns a vector ordered oldest-to-newest. Each entry is the net change
/// in account balance during that calendar month.
async fn compute_monthly_net_flows(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    today: NaiveDate,
    months: u32,
) -> Result<Vec<Decimal>> {
    let mut flows = Vec::with_capacity(months as usize);

    for i in (0..months).rev() {
        let target_date = today
            .checked_sub_months(Months::new(i))
            .unwrap_or(today);
        let month_start = NaiveDate::from_ymd_opt(target_date.year(), target_date.month(), 1)
            .unwrap_or(target_date);
        let month_end = get_last_day_of_month(target_date.year(), target_date.month());

        let df = match calculator
            .compute_account_state(db, accounts, month_start, month_end)
            .await
        {
            Ok(df) => df,
            Err(_) => continue,
        };

        if df.height() == 0 {
            flows.push(Decimal::ZERO);
            continue;
        }

        match extract_net_flow_from_df(&df, month_start, month_end) {
            Ok(flow) => flows.push(flow),
            Err(_) => flows.push(Decimal::ZERO),
        }
    }

    Ok(flows)
}

/// Extracts the net flow from a DataFrame by comparing balance at the end vs start of the period.
fn extract_net_flow_from_df(
    df: &DataFrame,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Decimal> {
    let date_col = df.column("date")
        .map_err(|e| ComputeError::DataFrame(format!("Missing date column: {e}")))?;
    let balance_col = df.column("balance")
        .map_err(|e| ComputeError::DataFrame(format!("Missing balance column: {e}")))?;

    // Polars Date type stores days since Unix epoch (1970-01-01)
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let start_num = start_date.signed_duration_since(epoch).num_days();
    let end_num = end_date.signed_duration_since(epoch).num_days();

    let mut earliest_bal: Option<(i64, Decimal)> = None;
    let mut latest_bal: Option<(i64, Decimal)> = None;

    for i in 0..df.height() {
        let date = date_col
            .get(i)
            .map_err(|e| ComputeError::Series(format!("Error getting date at row {i}: {e}")))?
            .try_extract::<i64>()
            .map_err(|e| ComputeError::Series(format!("Error extracting date at row {i}: {e}")))?;

        if date < start_num || date > end_num {
            continue;
        }

        let bal_any = balance_col
            .get(i)
            .map_err(|e| ComputeError::Series(format!("Error getting balance at row {i}: {e}")))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str)
            .map_err(|e| ComputeError::Decimal(format!("Invalid balance '{bal_str}': {e}")))?;

        match &earliest_bal {
            Some((d, _)) if date < *d => earliest_bal = Some((date, bal)),
            None => earliest_bal = Some((date, bal)),
            _ => {}
        }
        match &latest_bal {
            Some((d, _)) if date > *d => latest_bal = Some((date, bal)),
            None => latest_bal = Some((date, bal)),
            _ => {}
        }
    }

    match (earliest_bal, latest_bal) {
        (Some((_, start)), Some((_, end))) => Ok(end - start),
        _ => Ok(Decimal::ZERO),
    }
}

/// Computes the population standard deviation of a slice of Decimals.
fn compute_stddev(values: &[Decimal]) -> Option<Decimal> {
    if values.len() < 2 {
        return None;
    }
    let n = Decimal::from(values.len() as i64);
    let mean: Decimal = values.iter().sum::<Decimal>() / n;
    let variance: Decimal = values.iter().map(|v| (*v - mean) * (*v - mean)).sum::<Decimal>() / n;

    // Approximate square root via Newton's method for Decimal
    decimal_sqrt(variance)
}

/// Approximate square root of a non-negative Decimal using Newton's method.
fn decimal_sqrt(value: Decimal) -> Option<Decimal> {
    if value.is_zero() {
        return Some(Decimal::ZERO);
    }
    if value.is_sign_negative() {
        return None;
    }

    let two = Decimal::from(2);
    let mut guess = value / two;
    // 20 iterations is more than enough for Decimal precision
    for _ in 0..20 {
        if guess.is_zero() {
            return Some(Decimal::ZERO);
        }
        let next = (guess + value / guess) / two;
        if (next - guess).abs() < Decimal::new(1, 10) {
            return Some(next);
        }
        guess = next;
    }
    Some(guess)
}

/// Dispatches to kind-specific metric computation based on the account kind.
async fn compute_kind_metrics(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    account: &account::Model,
    accounts: &[account::Model],
    today: NaiveDate,
    current_balance: Decimal,
) -> Result<Option<AccountKindMetricsDto>> {
    match account.account_kind {
        AccountKind::RealAccount => {
            let m = compute_operating_metrics(calculator, db, account, accounts, current_balance, today).await?;
            Ok(Some(AccountKindMetricsDto::Operating(m)))
        }
        AccountKind::Savings | AccountKind::Goal => {
            let m = compute_reserve_metrics(
                calculator, db, account, accounts, today, current_balance,
            )
                .await?;
            Ok(Some(AccountKindMetricsDto::Reserve(m)))
        }
        AccountKind::Investment => {
            let m = compute_investment_metrics(calculator, db, accounts, today, current_balance)
                .await?;
            Ok(Some(AccountKindMetricsDto::Investment(m)))
        }
        AccountKind::Debt => {
            let m = compute_debt_metrics(calculator, db, account, accounts, today, current_balance)
                .await?;
            Ok(Some(AccountKindMetricsDto::Debt(m)))
        }
        AccountKind::Other => Ok(None),
    }
}

/// Computes operating-account-specific metrics (buffer, sweep potential, coverage).
///
/// Sweep potential uses the projected end-of-month balance rather than the
/// current mid-month balance, so it reflects the surplus after all recurring
/// expenses for the month have been applied.
async fn compute_operating_metrics(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    account: &account::Model,
    accounts: &[account::Model],
    current_balance: Decimal,
    today: NaiveDate,
) -> Result<OperatingMetricsDto> {
    let operating_buffer = account.target_amount.map(|t| current_balance - t);

    // Sweep potential uses projected end-of-month balance so it accounts for
    // all remaining expenses in the current month.
    let end_of_month = get_last_day_of_month(today.year(), today.month());
    let projected_eom_balance = match calculator
        .compute_account_state(db, accounts, today, end_of_month)
        .await
    {
        Ok(df) => {
            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let eom_num = end_of_month.signed_duration_since(epoch).num_days();
            extract_balance_at_date(&df, eom_num).unwrap_or(current_balance)
        }
        Err(_) => current_balance,
    };

    let sweep_potential = account
        .target_amount
        .map(|t| Decimal::max(Decimal::ZERO, projected_eom_balance - t));

    let mandatory_coverage_months =
        compute_mandatory_coverage(db, account.id, current_balance, today).await?;

    Ok(OperatingMetricsDto {
        operating_buffer,
        sweep_potential,
        mandatory_coverage_months,
    })
}

/// Extracts the balance closest to (but not exceeding) the target date from a DataFrame.
fn extract_balance_at_date(df: &DataFrame, target_date_num: i64) -> Option<Decimal> {
    let date_col = df.column("date").ok()?;
    let balance_col = df.column("balance").ok()?;

    let mut best: Option<(i64, Decimal)> = None;

    for i in 0..df.height() {
        let date = date_col.get(i).ok()?.try_extract::<i64>().ok()?;
        if date > target_date_num {
            continue;
        }
        let bal_any = balance_col.get(i).ok()?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str).ok()?;

        match &best {
            Some((d, _)) if date > *d => best = Some((date, bal)),
            None => best = Some((date, bal)),
            _ => {}
        }
    }

    best.map(|(_, bal)| bal)
}

/// Computes how many months of mandatory (recurring) outflows the current balance covers.
async fn compute_mandatory_coverage(
    db: &DatabaseConnection,
    account_id: i32,
    current_balance: Decimal,
    today: NaiveDate,
) -> Result<Option<Decimal>> {
    let all_recurring = recurring_transaction::Entity::find()
        .filter(recurring_transaction::Column::TargetAccountId.eq(account_id))
        .all(db)
        .await?;
    let recurring = filter_active_recurring(&all_recurring, today);

    let monthly_outflow: Decimal = recurring
        .iter()
        .filter(|r| r.amount.is_sign_negative())
        .map(|r| monthly_equivalent(r.amount.abs(), &r.period))
        .sum();

    if monthly_outflow.is_zero() {
        return Ok(None);
    }

    Ok(Some(current_balance / monthly_outflow))
}

/// Converts a per-period amount to a monthly equivalent.
pub fn monthly_equivalent(amount: Decimal, period: &recurring_transaction::RecurrencePeriod) -> Decimal {
    use recurring_transaction::RecurrencePeriod::*;
    let factor = match period {
        Daily => Decimal::new(3044, 2),       // ~30.44 days/month
        Weekly => Decimal::new(5218, 2) / Decimal::from(12), // 52.18/12 ≈ 4.348
        WorkDay => Decimal::new(2174, 2),     // ~21.74 work days/month
        Monthly => Decimal::ONE,
        Quarterly => Decimal::ONE / Decimal::from(3),
        HalfYearly => Decimal::ONE / Decimal::from(6),
        Yearly => Decimal::ONE / Decimal::from(12),
    };
    amount * factor
}

/// Computes reserve (Savings/Goal) specific metrics.
async fn compute_reserve_metrics(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    account: &account::Model,
    accounts: &[account::Model],
    today: NaiveDate,
    _current_balance: Decimal,
) -> Result<ReserveMetricsDto> {
    let goal_reached_date = if let Some(target_amount) = account.target_amount {
        let end_date = NaiveDate::from_ymd_opt(today.year() + 5, 12, 31)
            .unwrap_or(today);
        match calculator
            .compute_account_state(db, accounts, today, end_date)
            .await
        {
            Ok(df) => account_stats::calculate_goal_reached_date(&df, target_amount)
                .ok()
                .flatten(),
            Err(e) => {
                warn!(account_id = account.id, error = %e, "Failed to compute goal reached date");
                None
            }
        }
    } else {
        None
    };

    // months_of_essential_coverage is set later during cross-account computation
    // because it requires knowledge of the global essential burn rate
    Ok(ReserveMetricsDto {
        goal_reached_date,
        months_of_essential_coverage: None,
    })
}

/// Computes investment-account-specific metrics.
async fn compute_investment_metrics(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    today: NaiveDate,
    current_balance: Decimal,
) -> Result<InvestmentMetricsDto> {
    // Net contributions = sum of all positive balance changes (inflows)
    // We approximate by computing the balance at the very beginning and comparing
    let start_date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let df = calculator
        .compute_account_state(db, accounts, start_date, today)
        .await;

    let net_contributions = match df {
        Ok(df) => compute_net_inflows(&df).ok(),
        Err(_) => None,
    };

    let gain_loss_absolute = net_contributions.map(|nc| current_balance - nc);
    let gain_loss_percent = net_contributions
        .filter(|nc| !nc.is_zero())
        .map(|nc| (current_balance - nc) / nc * Decimal::from(100));

    Ok(InvestmentMetricsDto {
        net_contributions,
        gain_loss_absolute,
        gain_loss_percent,
    })
}

/// Sums all positive balance deltas in a DataFrame as an approximation of net inflows.
fn compute_net_inflows(df: &DataFrame) -> Result<Decimal> {
    let date_col = df.column("date")
        .map_err(|e| ComputeError::DataFrame(format!("Missing date column: {e}")))?;
    let balance_col = df.column("balance")
        .map_err(|e| ComputeError::DataFrame(format!("Missing balance column: {e}")))?;

    let mut points: Vec<(i64, Decimal)> = Vec::with_capacity(df.height());
    for i in 0..df.height() {
        let date = date_col.get(i)
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?
            .try_extract::<i64>()
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?;
        let bal_any = balance_col.get(i)
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str)
            .map_err(|e| ComputeError::Decimal(format!("Invalid balance '{bal_str}': {e}")))?;
        points.push((date, bal));
    }

    points.sort_by_key(|(d, _)| *d);

    let mut total_inflows = Decimal::ZERO;
    for w in points.windows(2) {
        let delta = w[1].1 - w[0].1;
        if delta > Decimal::ZERO {
            total_inflows += delta;
        }
    }

    Ok(total_inflows)
}

/// Computes debt-account-specific metrics.
async fn compute_debt_metrics(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    account: &account::Model,
    accounts: &[account::Model],
    today: NaiveDate,
    current_balance: Decimal,
) -> Result<DebtMetricsDto> {
    let outstanding_principal = Some(current_balance.abs());

    // Required monthly payment from recurring transactions (active only)
    let all_recurring = recurring_transaction::Entity::find()
        .filter(recurring_transaction::Column::TargetAccountId.eq(account.id))
        .all(db)
        .await?;
    let recurring = filter_active_recurring(&all_recurring, today);

    let required_monthly_payment: Decimal = recurring
        .iter()
        .filter(|r| r.amount.is_sign_positive())
        .map(|r| monthly_equivalent(r.amount, &r.period))
        .sum();

    let required_monthly_payment = if required_monthly_payment.is_zero() {
        None
    } else {
        Some(required_monthly_payment)
    };

    // Debt-free date: first future date where balance >= 0
    let debt_free_date = {
        let end_date = NaiveDate::from_ymd_opt(today.year() + 30, 12, 31)
            .unwrap_or(today);
        match calculator
            .compute_account_state(db, accounts, today, end_date)
            .await
        {
            Ok(df) => account_stats::calculate_goal_reached_date(&df, Decimal::ZERO)
                .ok()
                .flatten(),
            Err(_) => None,
        }
    };

    Ok(DebtMetricsDto {
        outstanding_principal,
        required_monthly_payment,
        debt_free_date,
    })
}

fn get_last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monthly_equivalent() {
        use recurring_transaction::RecurrencePeriod::*;

        let amount = Decimal::from(1000);
        assert_eq!(monthly_equivalent(amount, &Monthly), Decimal::from(1000));

        let yearly = monthly_equivalent(amount, &Yearly);
        let expected_yearly = Decimal::from(1000) / Decimal::from(12);
        assert!((yearly - expected_yearly).abs() < Decimal::new(1, 10));

        let quarterly = monthly_equivalent(amount, &Quarterly);
        let expected_quarterly = Decimal::from(1000) / Decimal::from(3);
        assert!((quarterly - expected_quarterly).abs() < Decimal::new(1, 10));
    }

    #[test]
    fn test_compute_stddev_empty() {
        assert_eq!(compute_stddev(&[]), None);
        assert_eq!(compute_stddev(&[Decimal::from(5)]), None);
    }

    #[test]
    fn test_compute_stddev_identical_values() {
        let values = vec![Decimal::from(100), Decimal::from(100), Decimal::from(100)];
        let result = compute_stddev(&values);
        assert!(result.is_some());
        assert!(result.unwrap() < Decimal::new(1, 2));
    }

    #[test]
    fn test_compute_stddev_known_values() {
        // stddev of [2, 4, 4, 4, 5, 5, 7, 9] = 2.0
        let values: Vec<Decimal> = vec![2, 4, 4, 4, 5, 5, 7, 9]
            .into_iter()
            .map(Decimal::from)
            .collect();
        let result = compute_stddev(&values).unwrap();
        assert!((result - Decimal::from(2)).abs() < Decimal::new(1, 4));
    }

    #[test]
    fn test_decimal_sqrt() {
        let result = decimal_sqrt(Decimal::from(4)).unwrap();
        assert!((result - Decimal::from(2)).abs() < Decimal::new(1, 8));

        let result = decimal_sqrt(Decimal::from(9)).unwrap();
        assert!((result - Decimal::from(3)).abs() < Decimal::new(1, 8));

        assert_eq!(decimal_sqrt(Decimal::ZERO), Some(Decimal::ZERO));
        assert_eq!(decimal_sqrt(Decimal::from(-1)), None);
    }

    #[test]
    fn test_extract_net_flow_empty_df() {
        let df = df! {
            "date" => Vec::<i64>::new(),
            "balance" => Vec::<&str>::new(),
        }
            .unwrap();

        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        let result = extract_net_flow_from_df(&df, start, end).unwrap();
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn test_extract_net_flow_normal() {
        let d1 = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();

        let df = df! {
            "account_id" => &[1i32, 1],
            "date" => &[d1.num_days_from_ce() as i64, d2.num_days_from_ce() as i64],
            "balance" => &["10000", "12500"],
        }
            .unwrap();

        let result = extract_net_flow_from_df(&df, d1, d2).unwrap();
        assert_eq!(result, Decimal::from(2500));
    }
}
