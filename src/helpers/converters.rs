use common::{AccountStatePoint, AccountStateTimeseries};
use polars::prelude::AnyValue;
use std::str::FromStr;

/// Helper function to convert DataFrame to AccountStateTimeseries
pub fn convert_dataframe_to_timeseries(
    df: polars::prelude::DataFrame,
) -> Result<AccountStateTimeseries, String> {
    // Extract columns from DataFrame
    let account_id_col = df
        .column("account_id")
        .map_err(|e| format!("Missing account_id column: {}", e))?;
    let date_col = df
        .column("date")
        .map_err(|e| format!("Missing date column: {}", e))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| format!("Missing balance column: {}", e))?;

    let mut data_points = Vec::new();

    // Iterate through rows and create AccountStatePoint objects
    for i in 0..df.height() {
        let account_id = account_id_col
            .get(i)
            .map_err(|e| format!("Error getting account_id at row {}: {}", i, e))?
            .try_extract::<i32>()
            .map_err(|e| format!("Error extracting account_id as i32 at row {}: {}", i, e))?;

        let date = date_col
            .get(i)
            .map_err(|e| format!("Error getting date at row {}: {}", i, e))?
            .try_extract::<i64>()
            .map_err(|e| format!("Error extracting date as i64 at row {}: {}", i, e))?;
        let naive_date = chrono::NaiveDate::from_num_days_from_ce_opt(date as i32)
            .ok_or_else(|| format!("Invalid date value at row {}: {}", i, date))?;

        let balance_str = match balance_col
            .get(i)
            .map_err(|e| format!("Error getting balance at row {}: {}", i, e))?
        {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => format!("{}", other),
        };
        let balance = rust_decimal::Decimal::from_str(&balance_str).map_err(|e| {
            format!(
                "Error parsing balance '{}' at row {}: {}",
                balance_str, i, e
            )
        })?;

        data_points.push(AccountStatePoint::new(account_id, naive_date, balance));
    }

    Ok(AccountStateTimeseries::new(data_points))
}
