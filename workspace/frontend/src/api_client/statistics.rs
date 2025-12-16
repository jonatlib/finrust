use crate::api_client;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountStatistics {
    pub account_id: i32,
    pub min_state: Option<Decimal>,
    pub max_state: Option<Decimal>,
    pub average_expense: Option<Decimal>,
    pub average_income: Option<Decimal>,
    pub upcoming_expenses: Option<Decimal>,
    pub end_of_period_state: Option<Decimal>,
    pub goal_reached_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountStatisticsCollection {
    pub period: TimePeriod,
    pub statistics: Vec<AccountStatistics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimePeriod {
    Year(i32),
    Month { year: i32, month: u32 },
    DateRange { start: NaiveDate, end: NaiveDate },
}

pub async fn get_account_statistics(account_id: i32) -> Result<AccountStatisticsCollection, String> {
    get_account_statistics_with_ignored(account_id, false).await
}

pub async fn get_account_statistics_with_ignored(account_id: i32, include_ignored: bool) -> Result<AccountStatisticsCollection, String> {
    log::trace!("Fetching statistics for account ID: {} (include_ignored={})", account_id, include_ignored);
    let url = if include_ignored {
        format!("/accounts/{}/statistics?include_ignored=true", account_id)
    } else {
        format!("/accounts/{}/statistics", account_id)
    };
    let result = api_client::get::<AccountStatisticsCollection>(&url).await;

    if let Err(ref e) = result {
        log::error!("Failed to fetch account statistics: {}", e);
    } else {
        log::info!("Successfully fetched statistics for account ID: {}", account_id);
    }

    result
}

pub async fn get_all_accounts_statistics() -> Result<Vec<AccountStatisticsCollection>, String> {
    log::trace!("Fetching statistics for all accounts");
    let result = api_client::get::<Vec<AccountStatisticsCollection>>("/accounts/statistics").await;

    if let Err(ref e) = result {
        log::error!("Failed to fetch all accounts statistics: {}", e);
    } else {
        log::info!("Successfully fetched statistics for all accounts");
    }

    result
}
