use chrono::NaiveDate;
use tracing::{debug, trace};

/// Processes occurrences for a recurring item, handling past and future occurrences differently.
///
/// For balance calculator:
/// - Future occurrences (date >= today) are treated as if they were accounted on their date
/// - Past occurrences (date < today) with instances are included on their due date
/// - Past occurrences (date < today) without instances are ignored
///
/// Returns a vector of dates that should be included in the result.
pub fn process_occurrences<T>(
    occurrences: Vec<NaiveDate>,
    instances: &[T],
    today: NaiveDate,
    item_id: i32,
    instance_has_due_date: impl Fn(&T) -> NaiveDate,
) -> Vec<NaiveDate> {
    let mut result = Vec::new();

    for date in occurrences {
        if date >= today {
            // Future occurrences are treated as if they were accounted on their date
            trace!(
                "Adding future occurrence on {} for recurring item id={}",
                date, item_id
            );
            result.push(date);
        } else {
            // Past occurrences
            // Check if there's an instance for this date
            let instance = instances.iter().find(|i| instance_has_due_date(i) == date);

            if instance.is_some() {
                // If there's an instance, include it on its due date
                trace!(
                    "Adding past occurrence with instance on {} for recurring item id={}",
                    date, item_id
                );
                result.push(date);
            } else {
                // If no instance, ignore it
                trace!(
                    "Ignoring past occurrence without instance on {} for recurring item id={}",
                    date, item_id
                );
                // Do not add to result
            }
        }
    }

    result
}
