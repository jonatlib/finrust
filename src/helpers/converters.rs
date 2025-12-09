use chrono::{Duration, NaiveDate};
use common::{AccountStatePoint, AccountStateTimeseries};
use polars::prelude::{AnyValue, SortMultipleOptions};
use std::str::FromStr;

/// Helper function to convert DataFrame to AccountStateTimeseries
pub fn convert_dataframe_to_timeseries(
    mut df: polars::prelude::DataFrame,
) -> Result<AccountStateTimeseries, String> {
    df.sort_in_place(["account_id", "date"], SortMultipleOptions::default())
        .map_err(|e| format!("Missing account_id column: {}", e))?;

    // Extract columns from DataFrame
    let account_id_col = df
        .column("account_id")
        .map_err(|e| format!("Missing account_id column: {}", e))?;
    let date_col = df
        .column("date")
        .map_err(|e| format!("Missing date column: {}", e))?
        .date()
        .map_err(|e| format!("Column is not of type Date: {}", e))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| format!("Missing balance column: {}", e))?;
    // Create the epoch constant once to avoid re-creating it 1000s of times
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

    let mut data_points = Vec::new();

    // Iterate through rows and create AccountStatePoint objects
    for i in 0..df.height() {
        let account_id = account_id_col
            .get(i)
            .map_err(|e| format!("Error getting account_id at row {}: {}", i, e))?
            .try_extract::<i32>()
            .map_err(|e| format!("Error extracting account_id as i32 at row {}: {}", i, e))?;

        // Retrieve the integer value (days) directly from the typed array
        let days_since_epoch = date_col
            .get(i) // Returns Option<i32>
            .ok_or_else(|| format!("Null date at row {}", i))?;

        // Perform the math: 1970-01-01 + X days
        let naive_date = epoch
            .checked_add_signed(Duration::days(days_since_epoch as i64))
            .ok_or_else(|| format!("Date out of range at row {}", i))?;

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
