use chrono::{Datelike, Duration, NaiveDate};
use model::entities::recurring_transaction;

use super::days_in_month;

/// Generates occurrence dates for a recurring event within the given date range.
pub fn generate_occurrences(
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    period: &recurring_transaction::RecurrencePeriod,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Vec<NaiveDate> {
    let mut occurrences = Vec::new();
    let mut current_date = start_date;

    // Check if the event ends before the range starts
    if let Some(end) = end_date {
        if end < range_start {
            return occurrences;
        }
    }

    // Generate occurrences until we reach the end of the range or the end of the event
    while current_date <= range_end {
        if current_date >= range_start {
            occurrences.push(current_date);
        }

        // Calculate the next occurrence based on the period
        match period {
            recurring_transaction::RecurrencePeriod::Daily => {
                current_date = current_date.succ_opt().unwrap();
            }
            recurring_transaction::RecurrencePeriod::Weekly => {
                current_date = current_date + Duration::days(7);
            }
            recurring_transaction::RecurrencePeriod::WorkDay => {
                // Skip to the next work day (Monday-Friday)
                current_date = current_date.succ_opt().unwrap();
                while current_date.weekday().num_days_from_monday() >= 5 {
                    current_date = current_date.succ_opt().unwrap();
                }
            }
            recurring_transaction::RecurrencePeriod::Monthly => {
                // Add one month
                let year = current_date.year() + (current_date.month() / 12) as i32;
                let month = (current_date.month() % 12) + 1;
                let day = std::cmp::min(current_date.day(), days_in_month(year, month));
                current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            }
            recurring_transaction::RecurrencePeriod::Quarterly => {
                // Add three months
                let year = current_date.year() + (current_date.month() / 12) as i32;
                let month = ((current_date.month() - 1 + 3) % 12) + 1;
                let day = std::cmp::min(current_date.day(), days_in_month(year, month));
                current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            }
            recurring_transaction::RecurrencePeriod::HalfYearly => {
                // Add six months
                let year = current_date.year() + (current_date.month() / 12) as i32;
                let month = ((current_date.month() - 1 + 6) % 12) + 1;
                let day = std::cmp::min(current_date.day(), days_in_month(year, month));
                current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            }
            recurring_transaction::RecurrencePeriod::Yearly => {
                // Add one year
                let year = current_date.year() + 1;
                let month = current_date.month();
                let day = std::cmp::min(current_date.day(), days_in_month(year, month));
                current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            }
        }

        // Check if we've reached the end of the event
        if let Some(end) = end_date {
            if current_date > end {
                break;
            }
        }
    }

    occurrences
}