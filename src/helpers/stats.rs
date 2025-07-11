use crate::schemas::StatisticsQuery;
use chrono::Datelike;
use common::{AccountStatistics, TimePeriod};
use compute::{account::AccountStateCalculator, account_stats, default_compute};
use model::entities::account;
use sea_orm::DatabaseConnection;

/// Helper function to determine time period from query parameters
pub fn determine_time_period(query: &StatisticsQuery) -> TimePeriod {
    if let (Some(start), Some(end)) = (query.start_date, query.end_date) {
        TimePeriod::date_range(start, end)
    } else if let (Some(year), Some(month)) = (query.year, query.month) {
        TimePeriod::month(year, month)
    } else if let Some(year) = query.year {
        TimePeriod::year(year)
    } else {
        // Default to current year
        TimePeriod::year(chrono::Utc::now().year())
    }
}

/// Compute statistics for a single account for a given time period
pub async fn compute_account_statistics(
    db: &DatabaseConnection,
    account: &account::Model,
    period: &TimePeriod,
) -> Result<AccountStatistics, Box<dyn std::error::Error + Send + Sync>> {
    let accounts = vec![account.clone()];
    let compute = default_compute(None);
    let account_id = account.id;

    let statistics = match period {
        TimePeriod::Year(year) => {
            let min_stats = account_stats::min_state_in_year(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let max_stats = account_stats::max_state_in_year(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let avg_expense_stats = account_stats::average_expense_in_year(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let avg_income_stats = account_stats::average_income_in_year(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let upcoming_expenses_stats = account_stats::upcoming_expenses_until_year_end(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
                chrono::Utc::now().date_naive(),
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let end_of_period_stats = account_stats::end_of_year_state(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
            )
            .await
            .unwrap_or_else(|_| vec![]);

            AccountStatistics {
                account_id,
                min_state: min_stats.first().and_then(|s| s.min_state),
                max_state: max_stats.first().and_then(|s| s.max_state),
                average_expense: avg_expense_stats.first().and_then(|s| s.average_expense),
                average_income: avg_income_stats.first().and_then(|s| s.average_income),
                upcoming_expenses: upcoming_expenses_stats
                    .first()
                    .and_then(|s| s.upcoming_expenses),
                end_of_period_state: end_of_period_stats
                    .first()
                    .and_then(|s| s.end_of_period_state),
            }
        }
        TimePeriod::Month { year, month } => {
            let min_stats = account_stats::min_state_in_month(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
                *month,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let max_stats = account_stats::max_state_in_month(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
                *month,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let avg_expense_stats = account_stats::average_expense_in_month(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
                *month,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let avg_income_stats = account_stats::average_income_in_month(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
                *month,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let upcoming_expenses_stats = account_stats::upcoming_expenses_until_month_end(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
                *month,
                chrono::Utc::now().date_naive(),
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let end_of_period_stats = account_stats::end_of_month_state(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                *year,
                *month,
            )
            .await
            .unwrap_or_else(|_| vec![]);

            AccountStatistics {
                account_id,
                min_state: min_stats.first().and_then(|s| s.min_state),
                max_state: max_stats.first().and_then(|s| s.max_state),
                average_expense: avg_expense_stats.first().and_then(|s| s.average_expense),
                average_income: avg_income_stats.first().and_then(|s| s.average_income),
                upcoming_expenses: upcoming_expenses_stats
                    .first()
                    .and_then(|s| s.upcoming_expenses),
                end_of_period_state: end_of_period_stats
                    .first()
                    .and_then(|s| s.end_of_period_state),
            }
        }
        TimePeriod::DateRange { start, end: _ } => {
            let year = start.year();
            let min_stats = account_stats::min_state_in_year(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                year,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let max_stats = account_stats::max_state_in_year(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                year,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let avg_expense_stats = account_stats::average_expense_in_year(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                year,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let avg_income_stats = account_stats::average_income_in_year(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                year,
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let upcoming_expenses_stats = account_stats::upcoming_expenses_until_year_end(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                year,
                chrono::Utc::now().date_naive(),
            )
            .await
            .unwrap_or_else(|_| vec![]);
            let end_of_period_stats = account_stats::end_of_year_state(
                &compute as &dyn AccountStateCalculator,
                db,
                &accounts,
                year,
            )
            .await
            .unwrap_or_else(|_| vec![]);

            AccountStatistics {
                account_id,
                min_state: min_stats.first().and_then(|s| s.min_state),
                max_state: max_stats.first().and_then(|s| s.max_state),
                average_expense: avg_expense_stats.first().and_then(|s| s.average_expense),
                average_income: avg_income_stats.first().and_then(|s| s.average_income),
                upcoming_expenses: upcoming_expenses_stats
                    .first()
                    .and_then(|s| s.upcoming_expenses),
                end_of_period_state: end_of_period_stats
                    .first()
                    .and_then(|s| s.end_of_period_state),
            }
        }
    };

    Ok(statistics)
}
