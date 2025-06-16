//! This file serves as the root for all SeaORM entity modules.
//! We define the data models for the finance tracking application here.
//! The structure is inspired by the provided Django models but adapted
//! for Rust's type system and the SeaORM framework.

pub mod account;
pub mod account_allowed_user;
pub mod account_tag;
pub mod imported_transaction;
pub mod manual_account_state;
pub mod one_off_transaction;
pub mod one_off_transaction_tag;
pub mod recurring_income;
pub mod recurring_income_tag;
pub mod recurring_transaction;
pub mod recurring_transaction_tag;
pub mod tag;
pub mod user;

// Define join tables for many-to-many relationships.
// SeaORM uses these to understand how to link entities.
pub mod prelude {
    //! A prelude module for easy importing of all entities.
    pub use super::account::Entity as Account;
    pub use super::account_allowed_user::Entity as AccountAllowedUser;
    pub use super::account_tag::Entity as AccountTag;
    pub use super::imported_transaction::Entity as ImportedTransaction;
    pub use super::manual_account_state::Entity as ManualAccountState;
    pub use super::one_off_transaction::Entity as OneOffTransaction;
    pub use super::one_off_transaction_tag::Entity as OneOffTransactionTag;
    pub use super::recurring_income::Entity as RecurringIncome;
    pub use super::recurring_income_tag::Entity as RecurringIncomeTag;
    pub use super::recurring_transaction::Entity as RecurringTransaction;
    pub use super::recurring_transaction_tag::Entity as RecurringTransactionTag;
    pub use super::tag::Entity as Tag;
    pub use super::user::Entity as User;
}

#[cfg(test)]
mod test {
    use chrono::NaiveDate;
    use migration::{Migrator, MigratorTrait};
    use rust_decimal::Decimal;
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbErr,
        EntityTrait, QueryFilter, QuerySelect, Set,
    };

    use super::*;
    use prelude::*;

    async fn setup_db() -> Result<DatabaseConnection, DbErr> {
        // Connect to the SQLite database
        let db = Database::connect("sqlite::memory:").await?;

        // Enable foreign keys
        db.execute_unprepared("PRAGMA foreign_keys = ON;").await?;

        // Try to apply migrations first
        Migrator::up(&db, None).await.expect("Migrations failed.");
        Ok(db)
    }

    #[tokio::test]
    async fn test_entity_integration() -> Result<(), DbErr> {
        // Setup database
        let db = setup_db().await?;

        // Create users
        let user1 = user::ActiveModel {
            username: Set("user1".to_string()),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        let user2 = user::ActiveModel {
            username: Set("user2".to_string()),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create tags
        let tag1 = tag::ActiveModel {
            name: Set("Groceries".to_string()),
            description: Set(Some("Food and household items".to_string())),
            parent_id: Set(None),
            ledger_name: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        let tag2 = tag::ActiveModel {
            name: Set("Utilities".to_string()),
            description: Set(Some("Bills for utilities".to_string())),
            parent_id: Set(None),
            ledger_name: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create accounts
        let account1 = account::ActiveModel {
            name: Set("Checking".to_string()),
            description: Set(Some("Main checking account".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(user1.id),
            include_in_statistics: Set(true),
            ledger_name: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        let account2 = account::ActiveModel {
            name: Set("Savings".to_string()),
            description: Set(Some("Savings account".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(user1.id),
            include_in_statistics: Set(true),
            ledger_name: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Link account to tag
        let account_tag = account_tag::ActiveModel {
            account_id: Set(account1.id),
            tag_id: Set(tag1.id),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create one-off transaction
        let one_off_tx = one_off_transaction::ActiveModel {
            name: Set("Grocery shopping".to_string()),
            description: Set(Some("Weekly grocery run".to_string())),
            amount: Set(Decimal::new(-5000, 2)), // -50.00
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 15).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(account1.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Link one-off transaction to tag
        let one_off_tx_tag = one_off_transaction_tag::ActiveModel {
            transaction_id: Set(one_off_tx.id),
            tag_id: Set(tag1.id),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create recurring transaction
        let recurring_tx = recurring_transaction::ActiveModel {
            name: Set("Rent payment".to_string()),
            description: Set(Some("Monthly rent".to_string())),
            amount: Set(Decimal::new(-120000, 2)), // -1200.00
            start_date: Set(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()),
            end_date: Set(None),
            period: Set(recurring_transaction::RecurrencePeriod::Monthly),
            include_in_statistics: Set(true),
            target_account_id: Set(account1.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Link recurring transaction to tag
        let recurring_tx_tag = recurring_transaction_tag::ActiveModel {
            transaction_id: Set(recurring_tx.id),
            tag_id: Set(tag2.id),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create a transfer transaction
        let transfer_tx = one_off_transaction::ActiveModel {
            name: Set("Transfer to savings".to_string()),
            description: Set(Some("Monthly savings transfer".to_string())),
            amount: Set(Decimal::new(-50000, 2)), // -500.00
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(account2.id),
            source_account_id: Set(Some(account1.id)),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Allow user2 to access account1
        let account_allowed_user = account_allowed_user::ActiveModel {
            account_id: Set(account1.id),
            user_id: Set(user2.id),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create manual account state
        let manual_state = manual_account_state::ActiveModel {
            account_id: Set(account1.id),
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()),
            amount: Set(Decimal::new(245000, 2)), // 2450.00
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create imported transaction
        let imported_tx = imported_transaction::ActiveModel {
            account_id: Set(account1.id),
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 20).unwrap()),
            description: Set("Imported grocery purchase".to_string()),
            amount: Set(Decimal::new(-4500, 2)), // -45.00
            import_hash: Set("abc123".to_string()),
            raw_data: Set(None),
            reconciled_one_off_transaction_id: Set(Some(one_off_tx.id)),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create recurring income
        let recurring_income = recurring_income::ActiveModel {
            name: Set("Salary".to_string()),
            description: Set(Some("Monthly salary payment".to_string())),
            amount: Set(Decimal::new(300000, 2)), // 3000.00
            start_date: Set(NaiveDate::from_ymd_opt(2023, 1, 25).unwrap()),
            end_date: Set(None),
            period: Set(recurring_transaction::RecurrencePeriod::Monthly),
            include_in_statistics: Set(true),
            target_account_id: Set(account1.id),
            source_name: Set(Some("Employer Inc.".to_string())),
            ledger_name: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Link recurring income to tag
        let recurring_income_tag = recurring_income_tag::ActiveModel {
            income_id: Set(recurring_income.id),
            tag_id: Set(tag2.id),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Read back and verify data

        // Verify users
        let users = User::find().all(&db).await?;
        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|u| u.username == "user1"));
        assert!(users.iter().any(|u| u.username == "user2"));

        // Verify accounts
        let accounts = Account::find().all(&db).await?;
        assert_eq!(accounts.len(), 2);
        assert!(accounts.iter().any(|a| a.name == "Checking"));
        assert!(accounts.iter().any(|a| a.name == "Savings"));

        // Verify tags
        let tags = Tag::find().all(&db).await?;
        assert_eq!(tags.len(), 2);
        assert!(tags.iter().any(|t| t.name == "Groceries"));
        assert!(tags.iter().any(|t| t.name == "Utilities"));

        // Verify one-off transactions
        let one_off_txs = OneOffTransaction::find().all(&db).await?;
        assert_eq!(one_off_txs.len(), 2);
        assert!(one_off_txs.iter().any(|t| t.name == "Grocery shopping"));
        assert!(one_off_txs.iter().any(|t| t.name == "Transfer to savings"));

        // Verify recurring transactions
        let recurring_txs = RecurringTransaction::find().all(&db).await?;
        assert_eq!(recurring_txs.len(), 1);
        assert_eq!(recurring_txs[0].name, "Rent payment");

        // Verify account-tag relationship
        let account_tags = AccountTag::find().all(&db).await?;
        assert_eq!(account_tags.len(), 1);
        assert_eq!(account_tags[0].account_id, account1.id);
        assert_eq!(account_tags[0].tag_id, tag1.id);

        // Verify one-off transaction-tag relationship
        let one_off_tx_tags = OneOffTransactionTag::find().all(&db).await?;
        assert_eq!(one_off_tx_tags.len(), 1);
        assert_eq!(one_off_tx_tags[0].transaction_id, one_off_tx.id);
        assert_eq!(one_off_tx_tags[0].tag_id, tag1.id);

        // Verify recurring transaction-tag relationship
        let recurring_tx_tags = RecurringTransactionTag::find().all(&db).await?;
        assert_eq!(recurring_tx_tags.len(), 1);
        assert_eq!(recurring_tx_tags[0].transaction_id, recurring_tx.id);
        assert_eq!(recurring_tx_tags[0].tag_id, tag2.id);

        // Verify account allowed users
        let account_allowed_users = AccountAllowedUser::find().all(&db).await?;
        assert_eq!(account_allowed_users.len(), 1);
        assert_eq!(account_allowed_users[0].account_id, account1.id);
        assert_eq!(account_allowed_users[0].user_id, user2.id);

        // Verify manual account states
        let manual_states = ManualAccountState::find().all(&db).await?;
        assert_eq!(manual_states.len(), 1);
        assert_eq!(manual_states[0].account_id, account1.id);
        assert_eq!(manual_states[0].amount, Decimal::new(245000, 2));

        // Verify imported transactions
        let imported_txs = ImportedTransaction::find().all(&db).await?;
        assert_eq!(imported_txs.len(), 1);
        assert_eq!(imported_txs[0].account_id, account1.id);
        assert_eq!(imported_txs[0].description, "Imported grocery purchase");
        assert_eq!(imported_txs[0].amount, Decimal::new(-4500, 2));
        assert_eq!(imported_txs[0].reconciled_one_off_transaction_id, Some(one_off_tx.id));

        // Verify recurring incomes
        let recurring_incomes = RecurringIncome::find().all(&db).await?;
        assert_eq!(recurring_incomes.len(), 1);
        assert_eq!(recurring_incomes[0].name, "Salary");
        assert_eq!(recurring_incomes[0].amount, Decimal::new(300000, 2));
        assert_eq!(recurring_incomes[0].target_account_id, account1.id);

        // Verify recurring income-tag relationship
        let recurring_income_tags = RecurringIncomeTag::find().all(&db).await?;
        assert_eq!(recurring_income_tags.len(), 1);
        assert_eq!(recurring_income_tags[0].income_id, recurring_income.id);
        assert_eq!(recurring_income_tags[0].tag_id, tag2.id);

        // Test relationships using Related trait

        // Get tags for account1 through account_tag
        let account1_tags = Tag::find()
            .join_as(
                sea_orm::JoinType::InnerJoin,
                tag::Entity::belongs_to(account_tag::Entity)
                    .from(tag::Column::Id)
                    .to(account_tag::Column::TagId)
                    .into(),
                account_tag::Entity,
            )
            .filter(account_tag::Column::AccountId.eq(account1.id))
            .all(&db)
            .await?;

        assert_eq!(account1_tags.len(), 1);
        assert_eq!(account1_tags[0].id, tag1.id);

        // Get transactions for account1
        let account1_txs = OneOffTransaction::find()
            .filter(one_off_transaction::Column::TargetAccountId.eq(account1.id))
            .all(&db)
            .await?;

        assert_eq!(account1_txs.len(), 1);
        assert_eq!(account1_txs[0].id, one_off_tx.id);

        Ok(())
    }
}
