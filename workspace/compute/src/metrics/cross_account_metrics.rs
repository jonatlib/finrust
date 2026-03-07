//! Cross-account metrics computation.
//!
//! Aggregates financial data across all accounts to produce dashboard-level
//! metrics such as net worth, burn rates, savings rate, and liquidity ratios.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait};
use tracing::{debug, instrument, trace, warn};

use common::metrics::{AccountKindMetricsDto, AccountMetricsDto, DashboardMetricsDto, ReserveMetricsDto};
use model::entities::account::{self, AccountKind};
use model::entities::recurring_transaction;

use crate::account::AccountStateCalculator;
use crate::error::Result;
use crate::metrics::account_metrics;

use super::account_metrics::monthly_equivalent;

/// Computes the full dashboard of cross-account metrics.
///
/// This includes per-account metrics for every account and aggregated
/// household-level metrics (net worth, burn rates, cashflow, etc.).
#[instrument(skip(calculator, db))]
pub async fn compute_dashboard_metrics(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    today: NaiveDate,
) -> Result<DashboardMetricsDto> {
    debug!("Computing dashboard metrics");

    // Fetch all accounts included in statistics
    let all_accounts: Vec<account::Model> = account::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .filter(|a| a.include_in_statistics)
        .collect();

    trace!(account_count = all_accounts.len(), "Fetched accounts for dashboard");

    // Compute per-account metrics
    let mut account_metrics_list: Vec<AccountMetricsDto> =
        Vec::with_capacity(all_accounts.len());

    for account in &all_accounts {
        match account_metrics::compute_account_metrics(calculator, db, account, today).await {
            Ok(metrics) => account_metrics_list.push(metrics),
            Err(e) => {
                warn!(
                    account_id = account.id,
                    error = %e,
                    "Failed to compute metrics for account, skipping"
                );
            }
        }
    }

    // --- Aggregated metrics ---

    // Net worth: sum all balances (debt accounts are negative)
    let total_net_worth: Decimal = account_metrics_list
        .iter()
        .map(|m| m.current_balance)
        .sum();

    // Liquid net worth: RealAccount + Savings + Goal, minus debts
    let liquid_net_worth: Decimal = account_metrics_list
        .iter()
        .filter(|m| {
            matches!(
                m.account_kind.as_str(),
                "RealAccount" | "Savings" | "Goal" | "Debt"
            )
        })
        .map(|m| m.current_balance)
        .sum();

    // Recurring transactions for burn rate and income calculations
    let all_recurring: Vec<recurring_transaction::Model> =
        recurring_transaction::Entity::find().all(db).await?;

    // Monthly income: sum of positive recurring amounts, excluding internal transfers
    let monthly_income: Decimal = all_recurring
        .iter()
        .filter(|r| r.amount.is_sign_positive() && r.include_in_statistics && r.source_account_id.is_none())
        .map(|r| monthly_equivalent(r.amount, &r.period))
        .sum();

    // Full burn rate: sum of all negative recurring amounts (as positive),
    // excluding internal transfers (those with source_account_id)
    let full_burn_rate: Decimal = all_recurring
        .iter()
        .filter(|r| r.amount.is_sign_negative() && r.include_in_statistics && r.source_account_id.is_none())
        .map(|r| monthly_equivalent(r.amount.abs(), &r.period))
        .sum();

    // Essential burn rate: recurring expenses on RealAccount targets only
    // (these represent core operating expenses)
    let real_account_ids: Vec<i32> = all_accounts
        .iter()
        .filter(|a| a.account_kind == AccountKind::RealAccount)
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

    // Free cashflow
    let free_cashflow = monthly_income - full_burn_rate;

    // Savings rate
    let savings_rate = if !monthly_income.is_zero() {
        Some((monthly_income - full_burn_rate) / monthly_income)
    } else {
        None
    };

    // Goal engine: monthly contributions to Savings + Investment + Goal accounts
    let wealth_account_ids: Vec<i32> = all_accounts
        .iter()
        .filter(|a| {
            matches!(
                a.account_kind,
                AccountKind::Savings | AccountKind::Investment | AccountKind::Goal
            )
        })
        .map(|a| a.id)
        .collect();

    // Goal engine includes internal transfers TO wealth accounts (these are
    // intentional savings/investment contributions) but excludes external income
    // that happens to land on a wealth account
    let goal_engine: Decimal = all_recurring
        .iter()
        .filter(|r| {
            r.amount.is_sign_negative()
                && r.include_in_statistics
                && r.source_account_id
                .map_or(false, |src| wealth_account_ids.contains(&src))
        })
        .map(|r| monthly_equivalent(r.amount.abs(), &r.period))
        .sum();

    // Commitment ratio: fixed recurring obligations / net income
    // Only real expenses (not internal transfers)
    let fixed_obligations: Decimal = all_recurring
        .iter()
        .filter(|r| r.amount.is_sign_negative() && r.include_in_statistics && r.source_account_id.is_none())
        .map(|r| monthly_equivalent(r.amount.abs(), &r.period))
        .sum();

    let commitment_ratio = if !monthly_income.is_zero() {
        Some(fixed_obligations / monthly_income)
    } else {
        None
    };

    // Liquidity ratio: liquid assets / monthly essential burn (in months)
    let liquid_assets: Decimal = account_metrics_list
        .iter()
        .filter(|m| matches!(m.account_kind.as_str(), "RealAccount" | "Savings" | "Goal"))
        .map(|m| m.current_balance)
        .sum();

    let liquidity_ratio_months = if !essential_burn_rate.is_zero() {
        Some(liquid_assets / essential_burn_rate)
    } else {
        None
    };

    // Total debt burden: monthly debt payments / net income
    let debt_account_ids: Vec<i32> = all_accounts
        .iter()
        .filter(|a| a.account_kind == AccountKind::Debt)
        .map(|a| a.id)
        .collect();

    let monthly_debt_payments: Decimal = all_recurring
        .iter()
        .filter(|r| {
            r.amount.is_sign_positive()
                && r.include_in_statistics
                && debt_account_ids.contains(&r.target_account_id)
        })
        .map(|r| monthly_equivalent(r.amount, &r.period))
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
