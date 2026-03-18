//! Cross-account metrics computation.
//!
//! Aggregates financial data across all accounts to produce dashboard-level
//! metrics such as net worth, burn rates, savings rate, and liquidity ratios.
//!
//! The dashboard uses a *batch compute* strategy: all account balances are
//! computed in a single `compute_account_state` call covering the full
//! required date range, and per-account metrics are extracted from the
//! resulting DataFrame. Only kind-specific long-range projections (reserve
//! goal dates, investment history, debt payoff) require additional targeted
//! per-account calls.

use chrono::{Datelike, Months, NaiveDate};
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, instrument, trace, warn};

use common::metrics::{
    AccountKindMetricsDto, AccountMetricsDto, DashboardMetricsDto, DebtMetricsDto,
    InvestmentMetricsDto, OperatingMetricsDto, ReserveMetricsDto,
};
use model::entities::account::{self, AccountKind};
use model::entities::recurring_transaction;

use crate::account::AccountStateCalculator;
use crate::account_stats;
use crate::error::{ComputeError, Result};

use super::account_metrics::{compute_stddev, monthly_equivalent};
use super::filter_active_recurring;

// ---------------------------------------------------------------------------
// Batch DataFrame helpers
// ---------------------------------------------------------------------------

/// Pre-processed per-account balance time series extracted from a batch DF.
struct AccountBalanceSeries {
    /// Sorted (epoch-day, balance) pairs.
    points: Vec<(i64, Decimal)>,
}

impl AccountBalanceSeries {
    /// Balance at the latest date on or before `target_epoch`.
    fn balance_at(&self, target_epoch: i64) -> Option<Decimal> {
        let mut best: Option<(i64, Decimal)> = None;
        for &(d, b) in &self.points {
            if d > target_epoch {
                continue;
            }
            match best {
                Some((bd, _)) if d > bd => best = Some((d, b)),
                None => best = Some((d, b)),
                _ => {}
            }
        }
        best.map(|(_, b)| b)
    }

    /// Net flow (latest_balance − earliest_balance) within [start, end] (epoch days, inclusive).
    fn net_flow_between(&self, start: i64, end: i64) -> Decimal {
        let mut earliest: Option<(i64, Decimal)> = None;
        let mut latest: Option<(i64, Decimal)> = None;
        for &(d, b) in &self.points {
            if d < start || d > end {
                continue;
            }
            match earliest {
                Some((ed, _)) if d < ed => earliest = Some((d, b)),
                None => earliest = Some((d, b)),
                _ => {}
            }
            match latest {
                Some((ld, _)) if d > ld => latest = Some((d, b)),
                None => latest = Some((d, b)),
                _ => {}
            }
        }
        match (earliest, latest) {
            (Some((_, s)), Some((_, e))) => e - s,
            _ => Decimal::ZERO,
        }
    }
}

/// Parses a batch DataFrame into per-account `AccountBalanceSeries`.
fn preprocess_batch_df(df: &DataFrame) -> Result<HashMap<i32, AccountBalanceSeries>> {
    let account_col = df
        .column("account_id")
        .or_else(|_| df.column("account"))
        .map_err(|e| ComputeError::DataFrame(format!("Missing account column: {e}")))?;
    let date_col = df
        .column("date")
        .map_err(|e| ComputeError::DataFrame(format!("Missing date column: {e}")))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| ComputeError::DataFrame(format!("Missing balance column: {e}")))?;

    let mut map: HashMap<i32, Vec<(i64, Decimal)>> = HashMap::new();

    for i in 0..df.height() {
        let aid = account_col
            .get(i)
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?
            .try_extract::<i32>()
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?;
        let date = date_col
            .get(i)
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?
            .try_extract::<i64>()
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?;
        let bal_any = balance_col
            .get(i)
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str)
            .map_err(|e| ComputeError::Decimal(format!("'{bal_str}': {e}")))?;
        map.entry(aid).or_default().push((date, bal));
    }

    let mut result = HashMap::with_capacity(map.len());
    for (aid, mut pts) in map {
        pts.sort_by_key(|(d, _)| *d);
        result.insert(aid, AccountBalanceSeries { points: pts });
    }
    Ok(result)
}

fn epoch_day(d: NaiveDate) -> i64 {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    d.signed_duration_since(epoch).num_days()
}

fn get_last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
}

/// Extracts monthly net flows for the last `months` months from the pre-processed data.
fn extract_monthly_net_flows(
    series: &AccountBalanceSeries,
    today: NaiveDate,
    months: u32,
) -> Vec<Decimal> {
    let mut flows = Vec::with_capacity(months as usize);
    for i in (0..months).rev() {
        let target_date = today
            .checked_sub_months(Months::new(i))
            .unwrap_or(today);
        let month_start = NaiveDate::from_ymd_opt(target_date.year(), target_date.month(), 1)
            .unwrap_or(target_date);
        let month_end = get_last_day_of_month(target_date.year(), target_date.month());
        flows.push(series.net_flow_between(epoch_day(month_start), epoch_day(month_end)));
    }
    flows
}

/// Account kind as a static string (matches existing behaviour).
fn kind_str(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::RealAccount => "RealAccount",
        AccountKind::Savings => "Savings",
        AccountKind::Investment => "Investment",
        AccountKind::Debt => "Debt",
        AccountKind::Other => "Other",
        AccountKind::Goal => "Goal",
        AccountKind::Allowance => "Allowance",
        AccountKind::Shared => "Shared",
        AccountKind::EmergencyFund => "EmergencyFund",
        AccountKind::Equity => "Equity",
        AccountKind::House => "House",
        AccountKind::Tax => "Tax",
    }
}

// ---------------------------------------------------------------------------
// Kind-specific metric builders (targeted per-account compute calls)
// ---------------------------------------------------------------------------

async fn compute_operating_metrics_batch(
    account: &account::Model,
    current_balance: Decimal,
    eom_balance: Decimal,
    db: &DatabaseConnection,
    today: NaiveDate,
) -> Result<OperatingMetricsDto> {
    let operating_buffer = account.target_amount.map(|t| current_balance - t);
    let sweep_potential = account
        .target_amount
        .map(|t| Decimal::max(Decimal::ZERO, eom_balance - t));

    let mandatory_coverage_months =
        compute_mandatory_coverage(db, account.id, current_balance, today).await?;

    Ok(OperatingMetricsDto {
        operating_buffer,
        sweep_potential,
        mandatory_coverage_months,
    })
}

async fn compute_reserve_metrics_batch(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    account: &account::Model,
    today: NaiveDate,
) -> Result<ReserveMetricsDto> {
    let goal_reached_date = if let Some(target_amount) = account.target_amount {
        let end_date =
            NaiveDate::from_ymd_opt(today.year() + 5, 12, 31).unwrap_or(today);
        let accounts = vec![account.clone()];
        match calculator
            .compute_account_state(db, &accounts, today, end_date)
            .await
        {
            Ok(df) => account_stats::calculate_goal_reached_date(&df, target_amount)
                .ok()
                .flatten(),
            Err(e) => {
                warn!(account_id = account.id, error = %e, "goal_reached_date failed");
                None
            }
        }
    } else {
        None
    };
    Ok(ReserveMetricsDto {
        goal_reached_date,
        months_of_essential_coverage: None,
    })
}

async fn compute_investment_metrics_batch(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    account: &account::Model,
    today: NaiveDate,
    current_balance: Decimal,
) -> Result<InvestmentMetricsDto> {
    let start_date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let accounts = vec![account.clone()];
    let net_contributions = match calculator
        .compute_account_state(db, &accounts, start_date, today)
        .await
    {
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

async fn compute_debt_metrics_batch(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    account: &account::Model,
    today: NaiveDate,
    current_balance: Decimal,
) -> Result<DebtMetricsDto> {
    let outstanding_principal = Some(current_balance.abs());

    let all_recurring = recurring_transaction::Entity::find()
        .filter(
            Condition::any()
                .add(recurring_transaction::Column::TargetAccountId.eq(account.id))
                .add(recurring_transaction::Column::SourceAccountId.eq(account.id)),
        )
        .all(db)
        .await?;
    let recurring = filter_active_recurring(&all_recurring, today);

    let required_monthly_payment: Decimal = recurring
        .iter()
        .filter(|r| {
            (r.target_account_id == account.id && r.amount.is_sign_positive())
                || (r.source_account_id == Some(account.id) && r.amount.is_sign_negative())
        })
        .map(|r| monthly_equivalent(r.amount.abs(), &r.period))
        .sum();
    let required_monthly_payment = if required_monthly_payment.is_zero() {
        None
    } else {
        Some(required_monthly_payment)
    };

    let debt_free_date = {
        let end_date =
            NaiveDate::from_ymd_opt(today.year() + 30, 12, 31).unwrap_or(today);
        let accounts = vec![account.clone()];
        match calculator
            .compute_account_state(db, &accounts, today, end_date)
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

/// Sums all positive balance deltas in a DataFrame as an approximation of net inflows.
fn compute_net_inflows(df: &DataFrame) -> Result<Decimal> {
    let date_col = df
        .column("date")
        .map_err(|e| ComputeError::DataFrame(format!("Missing date column: {e}")))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| ComputeError::DataFrame(format!("Missing balance column: {e}")))?;

    let mut points: Vec<(i64, Decimal)> = Vec::with_capacity(df.height());
    for i in 0..df.height() {
        let date = date_col
            .get(i)
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?
            .try_extract::<i64>()
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?;
        let bal_any = balance_col
            .get(i)
            .map_err(|e| ComputeError::Series(format!("row {i}: {e}")))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str)
            .map_err(|e| ComputeError::Decimal(format!("'{bal_str}': {e}")))?;
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

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Computes the full dashboard of cross-account metrics.
///
/// Uses a batch strategy: one `compute_account_state` call for ALL accounts
/// produces the base DataFrame, and per-account metrics are extracted from it.
/// Only long-range projections (reserve goals, investment history, debt payoff)
/// require additional targeted per-account calls.
#[instrument(skip(calculator, db))]
pub async fn compute_dashboard_metrics(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    today: NaiveDate,
) -> Result<DashboardMetricsDto> {
    debug!("Computing dashboard metrics (batch strategy)");

    let all_accounts: Vec<account::Model> = account::Entity::find().all(db).await?;
    trace!(account_count = all_accounts.len(), "Fetched accounts for dashboard");

    if all_accounts.is_empty() {
        return Ok(DashboardMetricsDto {
            total_net_worth: Decimal::ZERO,
            liquid_net_worth: Decimal::ZERO,
            non_liquid_net_worth: Decimal::ZERO,
            essential_burn_rate: Decimal::ZERO,
            full_burn_rate: Decimal::ZERO,
            free_cashflow: Decimal::ZERO,
            savings_rate: None,
            goal_engine: Decimal::ZERO,
            commitment_ratio: None,
            liquidity_ratio_months: None,
            total_debt_burden: None,
            account_metrics: vec![],
        });
    }

    // ── Batch balance computation ──────────────────────────────────────
    // state_at_date uses Jan-1-of-year..today; net flows need 6 months
    // back. We use the wider range and extract each metric from it.
    let year_start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
    let six_months_ago_start = today
        .checked_sub_months(Months::new(6))
        .map(|d| NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d))
        .unwrap_or(year_start);
    let range_start = std::cmp::min(year_start, six_months_ago_start);
    let end_of_month = get_last_day_of_month(today.year(), today.month());

    let batch_df = calculator
        .compute_account_state(db, &all_accounts, range_start, end_of_month)
        .await?;
    let account_data = preprocess_batch_df(&batch_df)?;

    debug!(
        rows = batch_df.height(),
        accounts = account_data.len(),
        "Batch DataFrame computed"
    );

    let today_epoch = epoch_day(today);
    let eom_epoch = epoch_day(end_of_month);

    // ── Per-account metrics ────────────────────────────────────────────
    let mut account_metrics_list: Vec<AccountMetricsDto> =
        Vec::with_capacity(all_accounts.len());

    for account in &all_accounts {
        let empty_series = AccountBalanceSeries { points: vec![] };
        let series = account_data.get(&account.id).unwrap_or(&empty_series);

        let current_balance = series.balance_at(today_epoch).unwrap_or(Decimal::ZERO);
        let target_balance = account.target_amount;
        let funding_ratio = target_balance
            .filter(|t| !t.is_zero())
            .map(|t| current_balance / t);

        let net_flows = extract_monthly_net_flows(series, today, 6);
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

        // Kind-specific metrics (only projections need extra compute calls)
        let kind_metrics = match account.account_kind {
            AccountKind::RealAccount | AccountKind::Allowance | AccountKind::Shared => {
                let eom_balance = series.balance_at(eom_epoch).unwrap_or(current_balance);
                let m = compute_operating_metrics_batch(
                    account,
                    current_balance,
                    eom_balance,
                    db,
                    today,
                )
                .await?;
                Some(AccountKindMetricsDto::Operating(m))
            }
            AccountKind::Savings | AccountKind::Goal | AccountKind::EmergencyFund => {
                let m =
                    compute_reserve_metrics_batch(calculator, db, account, today).await?;
                Some(AccountKindMetricsDto::Reserve(m))
            }
            AccountKind::Investment | AccountKind::Equity | AccountKind::House => {
                let m = compute_investment_metrics_batch(
                    calculator,
                    db,
                    account,
                    today,
                    current_balance,
                )
                .await?;
                Some(AccountKindMetricsDto::Investment(m))
            }
            AccountKind::Debt => {
                let m = compute_debt_metrics_batch(
                    calculator,
                    db,
                    account,
                    today,
                    current_balance,
                )
                .await?;
                Some(AccountKindMetricsDto::Debt(m))
            }
            AccountKind::Tax | AccountKind::Other => None,
        };

        account_metrics_list.push(AccountMetricsDto {
            account_id: account.id,
            account_kind: kind_str(account.account_kind).to_string(),
            current_balance,
            target_balance,
            funding_ratio,
            monthly_net_flow,
            three_month_avg_net_flow,
            flow_volatility,
            kind_metrics,
        });
    }

    // ── Aggregated cross-account metrics ──────────────────────────────

    let total_net_worth: Decimal = account_metrics_list
        .iter()
        .map(|m| m.current_balance)
        .sum();

    let liquid_lookup: HashMap<i32, bool> =
        all_accounts.iter().map(|a| (a.id, a.is_liquid)).collect();

    let liquid_net_worth: Decimal = account_metrics_list
        .iter()
        .filter(|m| *liquid_lookup.get(&m.account_id).unwrap_or(&true))
        .map(|m| m.current_balance)
        .sum();

    let non_liquid_net_worth: Decimal = account_metrics_list
        .iter()
        .filter(|m| !*liquid_lookup.get(&m.account_id).unwrap_or(&true))
        .map(|m| m.current_balance)
        .sum();

    // Recurring transactions for burn-rate calculations
    let all_recurring_raw: Vec<recurring_transaction::Model> =
        recurring_transaction::Entity::find().all(db).await?;
    let all_recurring = filter_active_recurring(&all_recurring_raw, today);

    trace!(
        total_in_db = all_recurring_raw.len(),
        active = all_recurring.len(),
        "Filtered active recurring transactions"
    );

    let full_burn_rate: Decimal = all_recurring
        .iter()
        .filter(|r| {
            r.amount.is_sign_negative()
                && r.include_in_statistics
                && r.source_account_id.is_none()
        })
        .map(|r| monthly_equivalent(r.amount.abs(), &r.period))
        .sum();

    let real_account_ids: Vec<i32> = all_accounts
        .iter()
        .filter(|a| {
            matches!(
                a.account_kind,
                AccountKind::RealAccount | AccountKind::Allowance | AccountKind::Shared
            )
        })
        .map(|a| a.id)
        .collect();

    let essential_burn_rate: Decimal = all_recurring
        .iter()
        .filter(|r| {
            r.amount.is_sign_negative()
                && r.include_in_statistics
                && r.source_account_id.is_none()
                && real_account_ids.contains(&r.target_account_id)
        })
        .map(|r| monthly_equivalent(r.amount.abs(), &r.period))
        .sum();

    trace!(%essential_burn_rate, %full_burn_rate, "Burn rates computed");

    let global_net_flow: Decimal = account_metrics_list
        .iter()
        .map(|m| m.monthly_net_flow.unwrap_or(Decimal::ZERO))
        .sum();
    let free_cashflow = global_net_flow;

    let monthly_income = free_cashflow + full_burn_rate;

    trace!(%monthly_income, %free_cashflow, "Income and free cashflow computed");

    let savings_rate = if monthly_income > Decimal::ZERO {
        Some(free_cashflow / monthly_income)
    } else {
        None
    };

    let wealth_account_ids: Vec<i32> = all_accounts
        .iter()
        .filter(|a| {
            matches!(
                a.account_kind,
                AccountKind::Savings
                    | AccountKind::Investment
                    | AccountKind::Goal
                    | AccountKind::EmergencyFund
                    | AccountKind::Equity
                    | AccountKind::House
            )
        })
        .map(|a| a.id)
        .collect();

    let goal_engine: Decimal = account_metrics_list
        .iter()
        .filter(|m| wealth_account_ids.contains(&m.account_id))
        .map(|m| Decimal::max(Decimal::ZERO, m.monthly_net_flow.unwrap_or(Decimal::ZERO)))
        .sum();

    let commitment_ratio = if monthly_income > Decimal::ZERO {
        Some(full_burn_rate / monthly_income)
    } else {
        None
    };

    let liquid_assets: Decimal = account_metrics_list
        .iter()
        .filter(|m| {
            *liquid_lookup.get(&m.account_id).unwrap_or(&true) && m.account_kind != "Debt"
        })
        .map(|m| m.current_balance)
        .sum();

    let liquidity_ratio_months = if !essential_burn_rate.is_zero() {
        Some(liquid_assets / essential_burn_rate)
    } else {
        None
    };

    let debt_account_ids: Vec<i32> = all_accounts
        .iter()
        .filter(|a| a.account_kind == AccountKind::Debt)
        .map(|a| a.id)
        .collect();

    let monthly_debt_payments: Decimal = all_recurring
        .iter()
        .filter(|r| {
            r.include_in_statistics
                && ((r.amount.is_sign_positive()
                    && debt_account_ids.contains(&r.target_account_id))
                    || (r.amount.is_sign_negative()
                        && r.source_account_id
                            .map_or(false, |sid| debt_account_ids.contains(&sid))))
        })
        .map(|r| monthly_equivalent(r.amount.abs(), &r.period))
        .sum();

    let total_debt_burden = if !monthly_income.is_zero() {
        Some(monthly_debt_payments / monthly_income)
    } else {
        None
    };

    // Enrich reserve accounts with months_of_essential_coverage
    if !essential_burn_rate.is_zero() {
        for am in &mut account_metrics_list {
            if let Some(AccountKindMetricsDto::Reserve(ref mut reserve)) = am.kind_metrics {
                reserve.months_of_essential_coverage =
                    Some(am.current_balance / essential_burn_rate);
            }
        }
    }

    debug!(
        %total_net_worth,
        %liquid_net_worth,
        %free_cashflow,
        "Dashboard metrics computed"
    );

    Ok(DashboardMetricsDto {
        total_net_worth,
        liquid_net_worth,
        non_liquid_net_worth,
        essential_burn_rate,
        full_burn_rate,
        free_cashflow,
        savings_rate,
        goal_engine,
        commitment_ratio,
        liquidity_ratio_months,
        total_debt_burden,
        account_metrics: account_metrics_list,
    })
}
