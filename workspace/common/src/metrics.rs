//! Financial metrics transport DTOs.
//!
//! This module provides Polars-free, serde-serializable structures for
//! per-account and cross-account financial metrics computed by the
//! `compute` crate and exposed through the API layer.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Universal metrics applicable to every account regardless of kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AccountMetricsDto {
    /// The account identifier
    pub account_id: i32,
    /// The account kind as a string (e.g. "RealAccount", "Savings", "Debt")
    pub account_kind: String,
    /// Current balance of the account
    pub current_balance: Decimal,
    /// Target balance configured on the account (buffer target, goal, budget)
    pub target_balance: Option<Decimal>,
    /// Ratio of current balance to target balance (0.0 – 1.0+)
    pub funding_ratio: Option<Decimal>,
    /// Net flow for the current month (inflows minus outflows)
    pub monthly_net_flow: Option<Decimal>,
    /// Rolling 3-month average of net flow
    pub three_month_avg_net_flow: Option<Decimal>,
    /// Standard deviation of monthly net flow (volatility)
    pub flow_volatility: Option<Decimal>,
    /// Kind-specific metrics (filled based on account kind)
    pub kind_metrics: Option<AccountKindMetricsDto>,
}

/// Kind-specific metrics that only apply to certain account types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum AccountKindMetricsDto {
    /// Metrics for operating / real accounts
    Operating(OperatingMetricsDto),
    /// Metrics for reserve accounts (Savings / Goal)
    Reserve(ReserveMetricsDto),
    /// Metrics for investment accounts
    Investment(InvestmentMetricsDto),
    /// Metrics for debt accounts
    Debt(DebtMetricsDto),
}

/// Metrics specific to operating (RealAccount) accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct OperatingMetricsDto {
    /// Amount above the buffer target (can be negative)
    pub operating_buffer: Option<Decimal>,
    /// Amount that can be swept without breaking the buffer: max(0, balance - target)
    pub sweep_potential: Option<Decimal>,
    /// How many months of mandatory outflows the current balance covers
    pub mandatory_coverage_months: Option<Decimal>,
}

/// Metrics specific to reserve (Savings / Goal) accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReserveMetricsDto {
    /// Projected date when the goal target will be reached
    pub goal_reached_date: Option<NaiveDate>,
    /// How many months of essential expenses this reserve covers
    pub months_of_essential_coverage: Option<Decimal>,
}

/// Metrics specific to investment accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct InvestmentMetricsDto {
    /// Total contributions (sum of all inflows)
    pub net_contributions: Option<Decimal>,
    /// Absolute gain or loss: current_balance - net_contributions
    pub gain_loss_absolute: Option<Decimal>,
    /// Percentage gain or loss
    pub gain_loss_percent: Option<Decimal>,
}

/// Metrics specific to debt accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DebtMetricsDto {
    /// Outstanding principal (positive value representing what is owed)
    pub outstanding_principal: Option<Decimal>,
    /// Required monthly payment from recurring transactions
    pub required_monthly_payment: Option<Decimal>,
    /// Projected date when the debt will be fully paid off
    pub debt_free_date: Option<NaiveDate>,
}

/// Cross-account dashboard metrics aggregated across all accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DashboardMetricsDto {
    /// Total net worth: all assets minus all debts
    pub total_net_worth: Decimal,
    /// Liquid net worth: RealAccount + Savings balances minus short-term debts
    pub liquid_net_worth: Decimal,
    /// Monthly cost of truly essential expenses
    pub essential_burn_rate: Decimal,
    /// Total monthly expenses across all accounts
    pub full_burn_rate: Decimal,
    /// Net income minus full burn rate
    pub free_cashflow: Decimal,
    /// (income - spending) / income
    pub savings_rate: Option<Decimal>,
    /// Monthly total going toward wealth building (EF + investments + sinking funds)
    pub goal_engine: Decimal,
    /// Fixed recurring obligations / net income
    pub commitment_ratio: Option<Decimal>,
    /// Liquid assets / monthly essential burn (in months)
    pub liquidity_ratio_months: Option<Decimal>,
    /// Sum of monthly debt payments / net income
    pub total_debt_burden: Option<Decimal>,
    /// Per-account metrics for all accounts
    pub account_metrics: Vec<AccountMetricsDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_metrics_dto_serialization() {
        let metrics = AccountMetricsDto {
            account_id: 1,
            account_kind: "RealAccount".to_string(),
            current_balance: Decimal::new(145000, 0),
            target_balance: Some(Decimal::new(100000, 0)),
            funding_ratio: Some(Decimal::new(145, 2)),
            monthly_net_flow: Some(Decimal::new(5000, 0)),
            three_month_avg_net_flow: Some(Decimal::new(4500, 0)),
            flow_volatility: Some(Decimal::new(1200, 0)),
            kind_metrics: Some(AccountKindMetricsDto::Operating(OperatingMetricsDto {
                operating_buffer: Some(Decimal::new(45000, 0)),
                sweep_potential: Some(Decimal::new(45000, 0)),
                mandatory_coverage_months: Some(Decimal::new(17, 1)),
            })),
        };

        let json = serde_json::to_string(&metrics).expect("serialization failed");
        let deserialized: AccountMetricsDto =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(metrics, deserialized);
    }

    #[test]
    fn test_dashboard_metrics_dto_serialization() {
        let dashboard = DashboardMetricsDto {
            total_net_worth: Decimal::new(1_500_000, 0),
            liquid_net_worth: Decimal::new(800_000, 0),
            essential_burn_rate: Decimal::new(85_000, 0),
            full_burn_rate: Decimal::new(120_000, 0),
            free_cashflow: Decimal::new(30_000, 0),
            savings_rate: Some(Decimal::new(20, 2)),
            goal_engine: Decimal::new(25_000, 0),
            commitment_ratio: Some(Decimal::new(55, 2)),
            liquidity_ratio_months: Some(Decimal::new(94, 1)),
            total_debt_burden: Some(Decimal::new(30, 2)),
            account_metrics: vec![],
        };

        let json = serde_json::to_string(&dashboard).expect("serialization failed");
        let deserialized: DashboardMetricsDto =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(dashboard, deserialized);
    }

    #[test]
    fn test_kind_metrics_tagged_enum_serialization() {
        let debt = AccountKindMetricsDto::Debt(DebtMetricsDto {
            outstanding_principal: Some(Decimal::new(2_500_000, 0)),
            required_monthly_payment: Some(Decimal::new(15_000, 0)),
            debt_free_date: NaiveDate::from_ymd_opt(2043, 6, 15),
        });

        let json = serde_json::to_string(&debt).expect("serialization failed");
        assert!(json.contains(r#""type":"Debt"#));
        let deserialized: AccountKindMetricsDto =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(debt, deserialized);
    }
}
