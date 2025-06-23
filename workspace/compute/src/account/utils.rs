use chrono::{Datelike, Duration, NaiveDate};
use model::entities::recurring_transaction;
use tracing::{debug, instrument, trace};

use super::days_in_month;

/// Generates occurrence dates for a recurring event within the given date range.
#[instrument(fields(start_date = %start_date, end_date = ?end_date, period = ?period, range_start = %range_start, range_end = %range_end
))]
pub fn generate_occurrences(
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    period: &recurring_transaction::RecurrencePeriod,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Vec<NaiveDate> {
    debug!(
        "Generating occurrences for period {:?} from {} to {}",
        period, range_start, range_end
    );
    let mut occurrences = Vec::new();
    let mut current_date = start_date;
    trace!("Initial date: {}", current_date);

    // Check if the event ends before the range starts
    if let Some(end) = end_date {
        if end < range_start {
            debug!(
                "Event ends at {} which is before range start {}, returning empty list",
                end, range_start
            );
            return occurrences;
        }
    }

    // Generate occurrences until we reach the end of the range or the end of the event
    while current_date <= range_end {
        if current_date >= range_start {
            trace!("Adding occurrence: {}", current_date);
            occurrences.push(current_date);
        } else {
            trace!(
                "Skipping date {} as it's before range start {}",
                current_date, range_start
            );
        }

        // Calculate the next occurrence based on the period
        trace!("Calculating next occurrence based on period: {:?}", period);
        match period {
            recurring_transaction::RecurrencePeriod::Daily => {
                current_date = current_date.succ_opt().unwrap();
                trace!("Daily: next date is {}", current_date);
            }
            recurring_transaction::RecurrencePeriod::Weekly => {
                current_date += Duration::days(7);
                trace!("Weekly: next date is {}", current_date);
            }
            recurring_transaction::RecurrencePeriod::WorkDay => {
                // Skip to the next work day (Monday-Friday)
                current_date = current_date.succ_opt().unwrap();
                while current_date.weekday().num_days_from_monday() >= 5 {
                    trace!("WorkDay: skipping weekend day {}", current_date);
                    current_date = current_date.succ_opt().unwrap();
                }
                trace!("WorkDay: next date is {}", current_date);
            }
            recurring_transaction::RecurrencePeriod::Monthly => {
                // Add one month
                let year = current_date.year() + (current_date.month() / 12) as i32;
                let month = (current_date.month() % 12) + 1;
                let day = std::cmp::min(current_date.day(), days_in_month(year, month));
                trace!(
                    "Monthly: calculating next date with year={}, month={}, day={}",
                    year, month, day
                );
                current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
                trace!("Monthly: next date is {}", current_date);
            }
            recurring_transaction::RecurrencePeriod::Quarterly => {
                // Add three months
                let year = current_date.year() + (current_date.month() / 12) as i32;
                let month = ((current_date.month() - 1 + 3) % 12) + 1;
                let day = std::cmp::min(current_date.day(), days_in_month(year, month));
                trace!(
                    "Quarterly: calculating next date with year={}, month={}, day={}",
                    year, month, day
                );
                current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
                trace!("Quarterly: next date is {}", current_date);
            }
            recurring_transaction::RecurrencePeriod::HalfYearly => {
                // Add six months
                let year = current_date.year() + (current_date.month() / 12) as i32;
                let month = ((current_date.month() - 1 + 6) % 12) + 1;
                let day = std::cmp::min(current_date.day(), days_in_month(year, month));
                trace!(
                    "HalfYearly: calculating next date with year={}, month={}, day={}",
                    year, month, day
                );
                current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
                trace!("HalfYearly: next date is {}", current_date);
            }
            recurring_transaction::RecurrencePeriod::Yearly => {
                // Add one year
                let year = current_date.year() + 1;
                let month = current_date.month();
                let day = std::cmp::min(current_date.day(), days_in_month(year, month));
                trace!(
                    "Yearly: calculating next date with year={}, month={}, day={}",
                    year, month, day
                );
                current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
                trace!("Yearly: next date is {}", current_date);
            }
        }

        // Check if we've reached the end of the event
        if let Some(end) = end_date {
            if current_date > end {
                debug!("Reached event end date {}, stopping", end);
                break;
            }
        }
    }

    debug!("Generated {} occurrences", occurrences.len());
    occurrences
}
