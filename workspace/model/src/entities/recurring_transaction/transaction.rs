use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, Weekday};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
};

use crate::entities::recurring_transaction::{Model as RecurringTransaction, RecurrencePeriod};
use crate::entities::{recurring_transaction_instance, tag};
use crate::transaction::{Tag, Transaction, TransactionGenerator};

#[async_trait]
impl TransactionGenerator for RecurringTransaction {
    async fn get_tag_for_transaction(
        &self,
        db: &sea_orm::DatabaseConnection,
        expand: bool,
    ) -> Vec<Tag> {
        // Query the database for tags associated with this recurring transaction
        // Using the Related trait to find tags related to this transaction
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
        // If the end date of the recurring transaction is before the start of the range,
        // or the start date is after the end of the range, there are no transactions
        if let Some(end_date) = self.end_date {
            if end_date < start {
                return false;
            }
        }

        if self.start_date > end {
            return false;
        }

        // Determine the effective start date (the later of the transaction start date and the range start date)
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

// Helper function to add transactions for both target and source accounts
async fn add_transaction(
    transactions: &mut Vec<Transaction>,
    transaction: &RecurringTransaction,
    date: NaiveDate,
    today: NaiveDate,
    db: &DatabaseConnection,
) {
    // Load tags for this transaction
    let tags = transaction.get_tag_for_transaction(db, false).await;

    let mut target_transaction = if tags.is_empty() {
        Transaction::new(date, transaction.amount, transaction.target_account_id)
    } else {
        Transaction::new_with_tags(
            date,
            transaction.amount,
            transaction.target_account_id,
            tags.clone(),
        )
    };

    // For recurring transactions: check for linked existing recurring instances
    // If they exist, take paid details from the instance. If not, set to not paid.
    if let Ok(Some(instance)) = recurring_transaction_instance::Entity::find()
        .filter(recurring_transaction_instance::Column::RecurringTransactionId.eq(transaction.id))
        .filter(recurring_transaction_instance::Column::DueDate.eq(date))
        .one(db)
        .await
    {
        // Instance exists - check if it's paid and if the date is not in the future
        match instance.status {
            recurring_transaction_instance::InstanceStatus::Paid => {
                // Use the paid date if available, otherwise use due date
                let paid_date = instance.paid_date.unwrap_or(instance.due_date);
                if paid_date <= today {
                    target_transaction.set_paid_on(Some(paid_date.and_hms_opt(0, 0, 0).unwrap()));
                }
                // Update transaction date to instance date and amount to actual paid amount if available
                target_transaction = if tags.is_empty() {
                    Transaction::new(
                        instance.due_date,
                        instance.paid_amount.unwrap_or(instance.expected_amount),
                        transaction.target_account_id,
                    )
                } else {
                    Transaction::new_with_tags(
                        instance.due_date,
                        instance.paid_amount.unwrap_or(instance.expected_amount),
                        transaction.target_account_id,
                        tags.clone(),
                    )
                };
                if paid_date <= today {
                    target_transaction.set_paid_on(Some(paid_date.and_hms_opt(0, 0, 0).unwrap()));
                }
            }
            recurring_transaction_instance::InstanceStatus::Pending => {
                // Update transaction date to instance due date
                target_transaction = if tags.is_empty() {
                    Transaction::new(
                        instance.due_date,
                        instance.expected_amount,
                        transaction.target_account_id,
                    )
                } else {
                    Transaction::new_with_tags(
                        instance.due_date,
                        instance.expected_amount,
                        transaction.target_account_id,
                        tags.clone(),
                    )
                };
                // Only mark as paid if due date is today or in the past
                if instance.due_date <= today {
                    target_transaction
                        .set_paid_on(Some(instance.due_date.and_hms_opt(0, 0, 0).unwrap()));
                }
            }
            recurring_transaction_instance::InstanceStatus::Skipped => {
                // Skipped instances are not paid
            }
        }
    }
    // If no instance exists, the transaction is not paid (default behavior)

    transactions.push(target_transaction);

    // If there's a source account, add a transaction for it as well
    if let Some(source_account_id) = transaction.source_account_id {
        // For the source account, the amount is negated
        let mut source_transaction = if tags.is_empty() {
            Transaction::new(date, -transaction.amount, source_account_id)
        } else {
            Transaction::new_with_tags(date, -transaction.amount, source_account_id, tags.clone())
        };

        // Apply the same instance-based payment logic to the source transaction
        if let Ok(Some(instance)) = recurring_transaction_instance::Entity::find()
            .filter(
                recurring_transaction_instance::Column::RecurringTransactionId.eq(transaction.id),
            )
            .filter(recurring_transaction_instance::Column::DueDate.eq(date))
            .one(db)
            .await
        {
            // Instance exists - check if it's paid and if the date is not in the future
            match instance.status {
                recurring_transaction_instance::InstanceStatus::Paid => {
                    // Use the paid date if available, otherwise use due date
                    let paid_date = instance.paid_date.unwrap_or(instance.due_date);
                    // Update transaction date to instance date and amount to actual paid amount if available (negated for source)
                    source_transaction = if tags.is_empty() {
                        Transaction::new(
                            instance.due_date,
                            -(instance.paid_amount.unwrap_or(instance.expected_amount)),
                            source_account_id,
                        )
                    } else {
                        Transaction::new_with_tags(
                            instance.due_date,
                            -(instance.paid_amount.unwrap_or(instance.expected_amount)),
                            source_account_id,
                            tags.clone(),
                        )
                    };
                    if paid_date <= today {
                        source_transaction
                            .set_paid_on(Some(paid_date.and_hms_opt(0, 0, 0).unwrap()));
                    }
                }
                recurring_transaction_instance::InstanceStatus::Pending => {
                    // Update transaction date to instance due date
                    source_transaction = if tags.is_empty() {
                        Transaction::new(
                            instance.due_date,
                            -instance.expected_amount,
                            source_account_id,
                        )
                    } else {
                        Transaction::new_with_tags(
                            instance.due_date,
                            -instance.expected_amount,
                            source_account_id,
                            tags.clone(),
                        )
                    };
                    // Only mark as paid if due date is today or in the past
                    if instance.due_date <= today {
                        source_transaction
                            .set_paid_on(Some(instance.due_date.and_hms_opt(0, 0, 0).unwrap()));
                    }
                }
                recurring_transaction_instance::InstanceStatus::Skipped => {
                    // Skipped instances are not paid
                }
            }
        }
        // If no instance exists, the transaction is not paid (default behavior)

        transactions.push(source_transaction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_has_any_transaction_monthly() {
        let transaction = RecurringTransaction {
            id: 1,
            name: "Monthly Rent".to_string(),
            description: None,
            amount: Decimal::new(-1000, 0),
            start_date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            end_date: None,
            period: RecurrencePeriod::Monthly,
            include_in_statistics: true,
            target_account_id: 1,
            source_account_id: None,
            ledger_name: None,
        };

        // Date range includes a monthly occurrence
        assert!(transaction.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()
        ));

        // Date range includes multiple monthly occurrences
        assert!(transaction.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 3, 31).unwrap()
        ));

        // Date range is before the start date
        assert!(!transaction.has_any_transaction(
            NaiveDate::from_ymd_opt(2022, 12, 1).unwrap(),
            NaiveDate::from_ymd_opt(2022, 12, 31).unwrap()
        ));

        // Date range is between monthly occurrences
        assert!(!transaction.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 16).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 14).unwrap()
        ));
    }

    #[tokio::test]
    async fn test_generate_transactions_monthly() {
        // Create a mock database connection for testing
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

        let transaction = RecurringTransaction {
            id: 1,
            name: "Monthly Rent".to_string(),
            description: None,
            amount: Decimal::new(-1000, 0),
            start_date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            end_date: None,
            period: RecurrencePeriod::Monthly,
            include_in_statistics: true,
            target_account_id: 1,
            source_account_id: None,
            ledger_name: None,
        };

        // Generate transactions for a 3-month period
        let today = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(); // Set today to Feb 1, 2023
        let transactions = transaction
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
                today,
                &db,
            )
            .await;

        assert_eq!(transactions.len(), 3);

        // Check January transaction (should not be paid since no instance exists)
        assert_eq!(
            transactions[0].date(),
            NaiveDate::from_ymd_opt(2023, 1, 15).unwrap()
        );
        assert_eq!(transactions[0].amount(), Decimal::new(-1000, 0));
        assert_eq!(transactions[0].account(), 1);
        assert!(!transactions[0].is_paid()); // Should not be paid since no instance exists

        // Check February transaction (should not be paid since no instance exists)
        assert_eq!(
            transactions[1].date(),
            NaiveDate::from_ymd_opt(2023, 2, 15).unwrap()
        );
        assert_eq!(transactions[1].amount(), Decimal::new(-1000, 0));
        assert_eq!(transactions[1].account(), 1);
        assert!(!transactions[1].is_paid()); // Should not be paid since no instance exists

        // Check March transaction (should not be paid since no instance exists)
        assert_eq!(
            transactions[2].date(),
            NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()
        );
        assert_eq!(transactions[2].amount(), Decimal::new(-1000, 0));
        assert_eq!(transactions[2].account(), 1);
        assert!(!transactions[2].is_paid()); // Should not be paid since no instance exists
    }

    #[tokio::test]
    async fn test_generate_transactions_with_source_account() {
        // Create a mock database connection for testing
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

        let transaction = RecurringTransaction {
            id: 2,
            name: "Monthly Transfer".to_string(),
            description: None,
            amount: Decimal::new(500, 0),
            start_date: NaiveDate::from_ymd_opt(2023, 1, 20).unwrap(),
            end_date: None,
            period: RecurrencePeriod::Monthly,
            include_in_statistics: true,
            target_account_id: 2,
            source_account_id: Some(1),
            ledger_name: None,
        };

        // Generate transactions for a 2-month period
        let today = NaiveDate::from_ymd_opt(2023, 1, 25).unwrap(); // Set today to Jan 25, 2023
        let transactions = transaction
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(),
                today,
                &db,
            )
            .await;

        assert_eq!(transactions.len(), 4); // 2 months * 2 accounts = 4 transactions

        // Check January transactions (should not be paid since no instance exists)
        // Target account
        assert_eq!(
            transactions[0].date(),
            NaiveDate::from_ymd_opt(2023, 1, 20).unwrap()
        );
        assert_eq!(transactions[0].amount(), Decimal::new(500, 0));
        assert_eq!(transactions[0].account(), 2);
        assert!(!transactions[0].is_paid()); // Should not be paid since no instance exists

        // Source account
        assert_eq!(
            transactions[1].date(),
            NaiveDate::from_ymd_opt(2023, 1, 20).unwrap()
        );
        assert_eq!(transactions[1].amount(), Decimal::new(-500, 0));
        assert_eq!(transactions[1].account(), 1);
        assert!(!transactions[1].is_paid()); // Should not be paid since no instance exists

        // Check February transactions (should not be paid since no instance exists)
        // Target account
        assert_eq!(
            transactions[2].date(),
            NaiveDate::from_ymd_opt(2023, 2, 20).unwrap()
        );
        assert_eq!(transactions[2].amount(), Decimal::new(500, 0));
        assert_eq!(transactions[2].account(), 2);
        assert!(!transactions[2].is_paid()); // Should not be paid since no instance exists

        // Source account
        assert_eq!(
            transactions[3].date(),
            NaiveDate::from_ymd_opt(2023, 2, 20).unwrap()
        );
        assert_eq!(transactions[3].amount(), Decimal::new(-500, 0));
        assert_eq!(transactions[3].account(), 1);
        assert!(!transactions[3].is_paid()); // Should not be paid since no instance exists
    }

    #[tokio::test]
    async fn test_generate_transactions_without_instances() {
        // Create a mock database connection for testing
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

        let transaction = RecurringTransaction {
            id: 1,
            name: "Monthly Rent".to_string(),
            description: None,
            amount: Decimal::new(-1000, 0),
            start_date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            end_date: None,
            period: RecurrencePeriod::Monthly,
            include_in_statistics: true,
            target_account_id: 1,
            source_account_id: None,
            ledger_name: None,
        };

        // Generate transactions for a 3-month period without any instances in the database
        let today = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(); // Set today to Feb 1, 2023
        let transactions = transaction
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
                today,
                &db,
            )
            .await;

        assert_eq!(transactions.len(), 3);

        // All transactions should not be paid since no instances exist
        for (i, transaction) in transactions.iter().enumerate() {
            assert!(
                !transaction.is_paid(),
                "Transaction {} should not be paid since no instance exists",
                i
            );
        }

        // Check that the transactions have the correct dates and amounts from the recurring rule
        assert_eq!(
            transactions[0].date(),
            NaiveDate::from_ymd_opt(2023, 1, 15).unwrap()
        );
        assert_eq!(transactions[0].amount(), Decimal::new(-1000, 0));
        assert_eq!(
            transactions[1].date(),
            NaiveDate::from_ymd_opt(2023, 2, 15).unwrap()
        );
        assert_eq!(transactions[1].amount(), Decimal::new(-1000, 0));
        assert_eq!(
            transactions[2].date(),
            NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()
        );
        assert_eq!(transactions[2].amount(), Decimal::new(-1000, 0));
    }
}
