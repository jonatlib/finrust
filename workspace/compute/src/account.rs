use chrono::{Datelike, NaiveDate};
use tracing::{debug, instrument, trace};

/// Returns the number of days in the given month using chrono.
#[instrument]
pub fn days_in_month(year: i32, month: u32) -> u32 {
    trace!("Calculating days in month for year={}, month={}", year, month);

    // Create a date for the first day of the next month
    let next_month_year = year + (month / 12) as i32;
    let next_month = (month % 12) + 1;
    trace!("Calculated next month: year={}, month={}", next_month_year, next_month);

    // Get the first day of the next month
    let first_day_next_month = NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap();
    trace!("First day of next month: {}", first_day_next_month);

    // Go back one day to get the last day of the current month
    let last_day_current_month = first_day_next_month.pred_opt().unwrap();
    trace!("Last day of current month: {}", last_day_current_month);

    // The day of the month is the number of days in the month
    let days = last_day_current_month.day();
    debug!("Days in month {}-{}: {}", year, month, days);
    days
}

pub mod balance;
pub mod forecast;
pub mod merge;
pub mod utils;
