use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, Weekday};
use sea_orm::{DatabaseConnection, EntityTrait, ModelTrait, RelationTrait};

use crate::entities::recurring_income::Model as RecurringIncome;
use crate::entities::recurring_transaction::RecurrencePeriod;
use crate::entities::{recurring_income_tag, tag};
use crate::transaction::{Tag, Transaction, TransactionGenerator};

#[async_trait]
impl TransactionGenerator for RecurringIncome {
    async fn get_tag_for_transaction(
        &self,
        db: &sea_orm::DatabaseConnection,
        expand: bool,
    ) -> Vec<Tag> {
        // Query the database for tags associated with this recurring income
        // Using the Related trait to find tags related to this income
        let tag_models = match self.find_related(tag::Entity).all(db).await {
            Ok(tags) => tags,
            Err(_) => return Vec::new(),
        };

        let mut result_tags = Vec::new();

        for tag_model in tag_models {
            let tag = Tag {
                id: tag_model.id,
                name: tag_model.name.clone(),
                description: tag_model.description.clone(),
            };

            if expand {
                // Expand this tag to include its parent hierarchy
                match tag_model.expand(db).await {
                    Ok(expanded_tags) => {
                        for expanded_tag in expanded_tags {
                            let expanded = Tag {
                                id: expanded_tag.id,
                                name: expanded_tag.name,
                                description: expanded_tag.description,
                            };
                            if !result_tags.iter().any(|t: &Tag| t.id == expanded.id) {
                                result_tags.push(expanded);
                            }
                        }
                    }
                    Err(_) => {
                        // If expansion fails, just add the original tag
                        if !result_tags.iter().any(|t: &Tag| t.id == tag.id) {
                            result_tags.push(tag);
                        }
                    }
                }
            } else {
                // Just add the tag without expansion
                result_tags.push(tag);
            }
        }

        result_tags
    }
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool {
        // If the end date of the recurring income is before the start of the range,
        // or the start date is after the end of the range, there are no transactions
        if let Some(end_date) = self.end_date {
            if end_date < start {
                return false;
            }
        }

        if self.start_date > end {
            return false;
        }

        // Determine the effective start date (the later of the income start date and the range start date)
        let effective_start = if self.start_date > start {
            self.start_date
        } else {
            start
        };

        // Check if there's at least one occurrence in the range
        match self.period {
            RecurrencePeriod::Daily => true, // At least one day must be in the range
            RecurrencePeriod::Weekly => {
                // Check if there's at least one matching weekday in the range
                let days_diff = (end.signed_duration_since(effective_start)).num_days();
                days_diff >= 0 && days_diff / 7 >= 0
            }
            RecurrencePeriod::WorkDay => {
                // Check if there's at least one workday in the range
                let mut current = effective_start;
                while current <= end {
                    let weekday = current.weekday();
                    if weekday != Weekday::Sat && weekday != Weekday::Sun {
                        return true;
                    }
                    current = current.succ_opt().unwrap_or(current);
                    if current == current.succ_opt().unwrap_or(current) {
                        break; // Avoid infinite loop
                    }
                }
                false
            }
            RecurrencePeriod::Monthly => {
                // Check if there's at least one matching day of month in the range
                let start_day = self.start_date.day();
                let mut current_month = effective_start.month();
                let mut current_year = effective_start.year();

                while NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap() <= end {
                    // Try to create a date with the same day in the current month
                    if let Some(date) =
                        NaiveDate::from_ymd_opt(current_year, current_month, start_day)
                    {
                        if date >= effective_start && date <= end {
                            return true;
                        }
                    }

                    // Move to the next month
                    current_month += 1;
                    if current_month > 12 {
                        current_month = 1;
                        current_year += 1;
                    }
                }
                false
            }
            RecurrencePeriod::Quarterly => {
                // Check if there's at least one matching day in a quarter in the range
                let start_day = self.start_date.day();
                let start_month = self.start_date.month();
                let quarter_month = ((start_month - 1) % 3) + 1; // 1, 2, or 3 representing the month within the quarter

                let mut current_month = effective_start.month();
                let mut current_year = effective_start.year();

                while NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap() <= end {
                    // Check if this is a matching month in a quarter (e.g., if start is in Feb, check Feb, May, Aug, Nov)
                    if current_month % 3 == quarter_month % 3 {
                        // Try to create a date with the same day in the current month
                        if let Some(date) =
                            NaiveDate::from_ymd_opt(current_year, current_month, start_day)
                        {
                            if date >= effective_start && date <= end {
                                return true;
                            }
                        }
                    }

                    // Move to the next month
                    current_month += 1;
                    if current_month > 12 {
                        current_month = 1;
                        current_year += 1;
                    }
                }
                false
            }
            RecurrencePeriod::HalfYearly => {
                // Check if there's at least one matching day in a half-year in the range
                let start_day = self.start_date.day();
                let start_month = self.start_date.month();
                let half_year_month = ((start_month - 1) % 6) + 1; // 1-6 representing the month within the half-year

                let mut current_month = effective_start.month();
                let mut current_year = effective_start.year();

                while NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap() <= end {
                    // Check if this is a matching month in a half-year
                    if current_month % 6 == half_year_month % 6 {
                        // Try to create a date with the same day in the current month
                        if let Some(date) =
                            NaiveDate::from_ymd_opt(current_year, current_month, start_day)
                        {
                            if date >= effective_start && date <= end {
                                return true;
                            }
                        }
                    }

                    // Move to the next month
                    current_month += 1;
                    if current_month > 12 {
                        current_month = 1;
                        current_year += 1;
                    }
                }
                false
            }
            RecurrencePeriod::Yearly => {
                // Check if there's at least one matching day and month in a year in the range
                let start_day = self.start_date.day();
                let start_month = self.start_date.month();

                let mut current_year = effective_start.year();

                while current_year <= end.year() {
                    // Try to create a date with the same day and month in the current year
                    if let Some(date) =
                        NaiveDate::from_ymd_opt(current_year, start_month, start_day)
                    {
                        if date >= effective_start && date <= end {
                            return true;
                        }
                    }

                    current_year += 1;
                }
                false
            }
        }
    }

    async fn generate_transactions(
        &self,
        start: NaiveDate,
        end: NaiveDate,
        today: NaiveDate,
        db: &DatabaseConnection,
    ) -> Vec<Transaction> {
        let mut transactions = Vec::new();

        // If there are no transactions in the range, return an empty vector
        if !self.has_any_transaction(start, end) {
            return transactions;
        }

        // Determine the effective start and end dates
        let effective_start = if self.start_date > start {
            self.start_date
        } else {
            start
        };
        let effective_end = if let Some(end_date) = self.end_date {
            if end_date < end { end_date } else { end }
        } else {
            end
        };

        // Generate transactions based on the recurrence period
        match self.period {
            RecurrencePeriod::Daily => {
                let mut current = effective_start;
                while current <= effective_end {
                    add_transaction(&mut transactions, self, current, today, db).await;

                    // Move to the next day
                    if let Some(next) = current.succ_opt() {
                        current = next;
                    } else {
                        break;
                    }
                }
            }
            RecurrencePeriod::Weekly => {
                let start_weekday = self.start_date.weekday();
                let mut current = effective_start;

                // Move to the first occurrence of the weekday in the range
                while current.weekday() != start_weekday {
                    if let Some(next) = current.succ_opt() {
                        current = next;
                    } else {
                        break;
                    }
                    if current > effective_end {
                        break;
                    }
                }

                // Generate transactions for each matching weekday
                while current <= effective_end {
                    add_transaction(&mut transactions, self, current, today, db).await;

                    // Move to the next week
                    for _ in 0..7 {
                        if let Some(next) = current.succ_opt() {
                            current = next;
                        } else {
                            break;
                        }
                    }
                }
            }
            RecurrencePeriod::WorkDay => {
                let mut current = effective_start;
                while current <= effective_end {
                    let weekday = current.weekday();
                    if weekday != Weekday::Sat && weekday != Weekday::Sun {
                        add_transaction(&mut transactions, self, current, today, db).await;
                    }

                    // Move to the next day
                    if let Some(next) = current.succ_opt() {
                        current = next;
                    } else {
                        break;
                    }
                }
            }
            RecurrencePeriod::Monthly => {
                let start_day = self.start_date.day();
                let mut current_month = effective_start.month();
                let mut current_year = effective_start.year();

                while NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap()
                    <= effective_end
                {
                    // Try to create a date with the same day in the current month
                    if let Some(date) =
                        NaiveDate::from_ymd_opt(current_year, current_month, start_day)
                    {
                        if date >= effective_start && date <= effective_end {
                            add_transaction(&mut transactions, self, date, today, db).await;
                        }
                    }

                    // Move to the next month
                    current_month += 1;
                    if current_month > 12 {
                        current_month = 1;
                        current_year += 1;
                    }
                }
            }
            RecurrencePeriod::Quarterly => {
                let start_day = self.start_date.day();
                let start_month = self.start_date.month();
                let quarter_month = ((start_month - 1) % 3) + 1; // 1, 2, or 3 representing the month within the quarter

                let mut current_month = effective_start.month();
                let mut current_year = effective_start.year();

                while NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap()
                    <= effective_end
                {
                    // Check if this is a matching month in a quarter
                    if current_month % 3 == quarter_month % 3 {
                        // Try to create a date with the same day in the current month
                        if let Some(date) =
                            NaiveDate::from_ymd_opt(current_year, current_month, start_day)
                        {
                            if date >= effective_start && date <= effective_end {
                                add_transaction(&mut transactions, self, date, today, db).await;
                            }
                        }
                    }

                    // Move to the next month
                    current_month += 1;
                    if current_month > 12 {
                        current_month = 1;
                        current_year += 1;
                    }
                }
            }
            RecurrencePeriod::HalfYearly => {
                let start_day = self.start_date.day();
                let start_month = self.start_date.month();
                let half_year_month = ((start_month - 1) % 6) + 1; // 1-6 representing the month within the half-year

                let mut current_month = effective_start.month();
                let mut current_year = effective_start.year();

                while NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap()
                    <= effective_end
                {
                    // Check if this is a matching month in a half-year
                    if current_month % 6 == half_year_month % 6 {
                        // Try to create a date with the same day in the current month
                        if let Some(date) =
                            NaiveDate::from_ymd_opt(current_year, current_month, start_day)
                        {
                            if date >= effective_start && date <= effective_end {
                                add_transaction(&mut transactions, self, date, today, db).await;
                            }
                        }
                    }

                    // Move to the next month
                    current_month += 1;
                    if current_month > 12 {
                        current_month = 1;
                        current_year += 1;
                    }
                }
            }
            RecurrencePeriod::Yearly => {
                let start_day = self.start_date.day();
                let start_month = self.start_date.month();

                let mut current_year = effective_start.year();

                while current_year <= effective_end.year() {
                    // Try to create a date with the same day and month in the current year
                    if let Some(date) =
                        NaiveDate::from_ymd_opt(current_year, start_month, start_day)
                    {
                        if date >= effective_start && date <= effective_end {
                            add_transaction(&mut transactions, self, date, today, db).await;
                        }
                    }

                    current_year += 1;
                }
            }
        }

        transactions
    }
}

// Helper function to add a transaction for the target account
async fn add_transaction(
    transactions: &mut Vec<Transaction>,
    income: &RecurringIncome,
    date: NaiveDate,
    today: NaiveDate,
    db: &DatabaseConnection,
) {
    // Load tags for this transaction
    let tags = income.get_tag_for_transaction(db, false).await;

    let mut transaction = if tags.is_empty() {
        Transaction::new(date, income.amount, income.target_account_id)
    } else {
        Transaction::new_with_tags(date, income.amount, income.target_account_id, tags)
    };

    // For recurring income: if the transaction date is today or in the past, mark as paid
    // TODO: In the future, this should check for linked existing recurring instances
    // If they exist, take paid details from the instance. If not, set to not paid.
    if date <= today {
        // Set paid_on to the transaction date at midnight (start of day)
        transaction.set_paid_on(Some(date.and_hms_opt(0, 0, 0).unwrap()));
    }

    transactions.push(transaction);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_has_any_transaction_monthly() {
        let income = RecurringIncome {
            id: 1,
            name: "Monthly Salary".to_string(),
            description: None,
            amount: Decimal::new(5000, 0),
            start_date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            end_date: None,
            period: RecurrencePeriod::Monthly,
            include_in_statistics: true,
            target_account_id: 1,
            source_name: Some("Employer".to_string()),
            ledger_name: None,
        };

        // Date range includes a monthly occurrence
        assert!(income.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()
        ));

        // Date range includes multiple monthly occurrences
        assert!(income.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 3, 31).unwrap()
        ));

        // Date range is before the start date
        assert!(!income.has_any_transaction(
            NaiveDate::from_ymd_opt(2022, 12, 1).unwrap(),
            NaiveDate::from_ymd_opt(2022, 12, 31).unwrap()
        ));

        // Date range is between monthly occurrences
        assert!(!income.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 16).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 14).unwrap()
        ));
    }

    #[tokio::test]
    async fn test_generate_transactions_monthly() {
        // Create a mock database connection for testing
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

        let income = RecurringIncome {
            id: 1,
            name: "Monthly Salary".to_string(),
            description: None,
            amount: Decimal::new(5000, 0),
            start_date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            end_date: None,
            period: RecurrencePeriod::Monthly,
            include_in_statistics: true,
            target_account_id: 1,
            source_name: Some("Employer".to_string()),
            ledger_name: None,
        };

        // Generate transactions for a 3-month period
        let today = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(); // Set today to Feb 1, 2023
        let transactions = income
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
                today,
                &db,
            )
            .await;

        assert_eq!(transactions.len(), 3);

        // Check January transaction (should be paid since it's before today)
        assert_eq!(
            transactions[0].date(),
            NaiveDate::from_ymd_opt(2023, 1, 15).unwrap()
        );
        assert_eq!(transactions[0].amount(), Decimal::new(5000, 0));
        assert_eq!(transactions[0].account(), 1);
        assert!(transactions[0].is_paid()); // Should be paid since Jan 15 < Feb 1 (today)

        // Check February transaction (should not be paid since it's after today)
        assert_eq!(
            transactions[1].date(),
            NaiveDate::from_ymd_opt(2023, 2, 15).unwrap()
        );
        assert_eq!(transactions[1].amount(), Decimal::new(5000, 0));
        assert_eq!(transactions[1].account(), 1);
        assert!(!transactions[1].is_paid()); // Should not be paid since Feb 15 > Feb 1 (today)

        // Check March transaction (should not be paid since it's after today)
        assert_eq!(
            transactions[2].date(),
            NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()
        );
        assert_eq!(transactions[2].amount(), Decimal::new(5000, 0));
        assert_eq!(transactions[2].account(), 1);
        assert!(!transactions[2].is_paid()); // Should not be paid since Mar 15 > Feb 1 (today)
    }
}
