use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use crate::api_client;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountStatePoint {
    pub account_id: i32,
    pub date: NaiveDate,
    pub balance: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountStateTimeseries {
    pub data_points: Vec<AccountStatePoint>,
    pub date_range: DateRange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

pub async fn get_account_timeseries(
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<AccountStateTimeseries, String> {
    get_account_timeseries_with_ignored(account_id, start_date, end_date, false).await
}

pub async fn get_account_timeseries_with_ignored(
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    include_ignored: bool,
) -> Result<AccountStateTimeseries, String> {
    get_account_timeseries_with_scenario(account_id, start_date, end_date, include_ignored, None).await
}

pub async fn get_account_timeseries_with_scenario(
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    include_ignored: bool,
    scenario_id: Option<i32>,
) -> Result<AccountStateTimeseries, String> {
    log::trace!("Fetching timeseries for account ID: {} from {} to {} (include_ignored={}, scenario_id={:?})",
        account_id, start_date, end_date, include_ignored, scenario_id);

    let mut url = format!(
        "/accounts/{}/timeseries?start_date={}&end_date={}",
        account_id, start_date, end_date
    );

    if include_ignored {
        url.push_str("&include_ignored=true");
    }

    if let Some(sid) = scenario_id {
        url.push_str(&format!("&scenario_id={}", sid));
    }

    let result = api_client::get::<AccountStateTimeseries>(&url).await;

    if let Err(ref e) = result {
        log::error!("Failed to fetch account timeseries: {}", e);
    } else {
        log::info!("Successfully fetched timeseries for account ID: {}", account_id);
    }

    result
}

pub async fn get_all_accounts_timeseries(
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<AccountStateTimeseries, String> {
    get_all_accounts_timeseries_with_scenario(start_date, end_date, None).await
}

pub async fn get_all_accounts_timeseries_with_scenario(
    start_date: NaiveDate,
    end_date: NaiveDate,
    scenario_id: Option<i32>,
) -> Result<AccountStateTimeseries, String> {
    log::trace!("Fetching timeseries for all accounts from {} to {} (scenario_id={:?})",
        start_date, end_date, scenario_id);

    let mut url = format!(
        "/accounts/timeseries?start_date={}&end_date={}",
        start_date, end_date
    );

    if let Some(sid) = scenario_id {
        url.push_str(&format!("&scenario_id={}", sid));
    }

    let result = api_client::get::<AccountStateTimeseries>(&url).await;

    if let Err(ref e) = result {
        log::error!("Failed to fetch all accounts timeseries: {}", e);
    } else {
        log::info!("Successfully fetched timeseries for all accounts");
    }

    result
}
