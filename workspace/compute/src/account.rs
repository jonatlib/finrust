use chrono::{Datelike, NaiveDate};

/// Returns the number of days in the given month using chrono.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    // Create a date for the first day of the next month
    let next_month_year = year + (month / 12) as i32;
    let next_month = (month % 12) + 1;

    // Get the first day of the next month
    let first_day_next_month = NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap();

    // Go back one day to get the last day of the current month
    let last_day_current_month = first_day_next_month.pred_opt().unwrap();

    // The day of the month is the number of days in the month
    last_day_current_month.day()
}

pub mod balance;
pub mod forecast;
pub mod merge;
pub mod utils;
