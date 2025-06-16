use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create users table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(pk_auto(Users::Id))
                    .col(string(Users::Username).unique_key())
                    .to_owned(),
            )
            .await?;

        // Create tags table
        manager
            .create_table(
                Table::create()
                    .table(Tags::Table)
                    .if_not_exists()
                    .col(pk_auto(Tags::Id))
                    .col(string(Tags::Name).unique_key())
                    .col(string_null(Tags::Description))
                    .col(integer_null(Tags::ParentId))
                    .col(string_null(Tags::LedgerName))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tag_parent")
                            .from(Tags::Table, Tags::ParentId)
                            .to(Tags::Table, Tags::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create accounts table
        manager
            .create_table(
                Table::create()
                    .table(Accounts::Table)
                    .if_not_exists()
                    .col(pk_auto(Accounts::Id))
                    .col(string(Accounts::Name))
                    .col(string_null(Accounts::Description))
                    .col(string(Accounts::CurrencyCode))
                    .col(integer(Accounts::OwnerId))
                    .col(boolean(Accounts::IncludeInStatistics).default(true))
                    .col(string_null(Accounts::LedgerName))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_account_owner")
                            .from(Accounts::Table, Accounts::OwnerId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create accounts_tags table (join table)
        manager
            .create_table(
                Table::create()
                    .table(AccountsTags::Table)
                    .if_not_exists()
                    .col(integer(AccountsTags::AccountId))
                    .col(integer(AccountsTags::TagId))
                    .primary_key(
                        Index::create()
                            .name("pk_accounts_tags")
                            .col(AccountsTags::AccountId)
                            .col(AccountsTags::TagId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_tags_account")
                            .from(AccountsTags::Table, AccountsTags::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_tags_tag")
                            .from(AccountsTags::Table, AccountsTags::TagId)
                            .to(Tags::Table, Tags::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create accounts_allowed_users table (join table)
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("accounts_allowed_users"))
                    .if_not_exists()
                    .col(integer(AccountAllowedUsers::AccountId))
                    .col(integer(AccountAllowedUsers::UserId))
                    .primary_key(
                        Index::create()
                            .name("pk_accounts_allowed_users")
                            .col(AccountAllowedUsers::AccountId)
                            .col(AccountAllowedUsers::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_allowed_users_account")
                            .from(Alias::new("accounts_allowed_users"), AccountAllowedUsers::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_allowed_users_user")
                            .from(Alias::new("accounts_allowed_users"), AccountAllowedUsers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create manual_account_states table
        manager
            .create_table(
                Table::create()
                    .table(ManualAccountStates::Table)
                    .if_not_exists()
                    .col(pk_auto(ManualAccountStates::Id))
                    .col(integer(ManualAccountStates::AccountId))
                    .col(date(ManualAccountStates::Date))
                    .col(decimal(ManualAccountStates::Amount).decimal_len(16, 4))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_manual_account_states_account")
                            .from(ManualAccountStates::Table, ManualAccountStates::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create one_off_transactions table
        manager
            .create_table(
                Table::create()
                    .table(OneOffTransactions::Table)
                    .if_not_exists()
                    .col(pk_auto(OneOffTransactions::Id))
                    .col(string(OneOffTransactions::Name))
                    .col(string_null(OneOffTransactions::Description))
                    .col(decimal(OneOffTransactions::Amount).decimal_len(16, 4))
                    .col(date(OneOffTransactions::Date))
                    .col(boolean(OneOffTransactions::IncludeInStatistics).default(true))
                    .col(integer(OneOffTransactions::TargetAccountId))
                    .col(integer_null(OneOffTransactions::SourceAccountId))
                    .col(string_null(OneOffTransactions::LedgerName))
                    .col(string_null(OneOffTransactions::LinkedImportId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_off_transactions_target_account")
                            .from(OneOffTransactions::Table, OneOffTransactions::TargetAccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_off_transactions_source_account")
                            .from(OneOffTransactions::Table, OneOffTransactions::SourceAccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create one_off_transactions_tags table (join table)
        manager
            .create_table(
                Table::create()
                    .table(OneOffTransactionsTags::Table)
                    .if_not_exists()
                    .col(integer(OneOffTransactionsTags::TransactionId))
                    .col(integer(OneOffTransactionsTags::TagId))
                    .primary_key(
                        Index::create()
                            .name("pk_one_off_transactions_tags")
                            .col(OneOffTransactionsTags::TransactionId)
                            .col(OneOffTransactionsTags::TagId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_off_transactions_tags_transaction")
                            .from(OneOffTransactionsTags::Table, OneOffTransactionsTags::TransactionId)
                            .to(OneOffTransactions::Table, OneOffTransactions::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_off_transactions_tags_tag")
                            .from(OneOffTransactionsTags::Table, OneOffTransactionsTags::TagId)
                            .to(Tags::Table, Tags::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create recurring_transactions table
        manager
            .create_table(
                Table::create()
                    .table(RecurringTransactions::Table)
                    .if_not_exists()
                    .col(pk_auto(RecurringTransactions::Id))
                    .col(string(RecurringTransactions::Name))
                    .col(string_null(RecurringTransactions::Description))
                    .col(decimal(RecurringTransactions::Amount).decimal_len(16, 4))
                    .col(date(RecurringTransactions::StartDate))
                    .col(date_null(RecurringTransactions::EndDate))
                    .col(string(RecurringTransactions::Period).string_len(1))
                    .col(boolean(RecurringTransactions::IncludeInStatistics).default(true))
                    .col(integer(RecurringTransactions::TargetAccountId))
                    .col(integer_null(RecurringTransactions::SourceAccountId))
                    .col(string_null(RecurringTransactions::LedgerName))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_transactions_target_account")
                            .from(RecurringTransactions::Table, RecurringTransactions::TargetAccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_transactions_source_account")
                            .from(RecurringTransactions::Table, RecurringTransactions::SourceAccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create recurring_transactions_tags table (join table)
        manager
            .create_table(
                Table::create()
                    .table(RecurringTransactionsTags::Table)
                    .if_not_exists()
                    .col(integer(RecurringTransactionsTags::TransactionId))
                    .col(integer(RecurringTransactionsTags::TagId))
                    .primary_key(
                        Index::create()
                            .name("pk_recurring_transactions_tags")
                            .col(RecurringTransactionsTags::TransactionId)
                            .col(RecurringTransactionsTags::TagId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_transactions_tags_transaction")
                            .from(RecurringTransactionsTags::Table, RecurringTransactionsTags::TransactionId)
                            .to(RecurringTransactions::Table, RecurringTransactions::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_transactions_tags_tag")
                            .from(RecurringTransactionsTags::Table, RecurringTransactionsTags::TagId)
                            .to(Tags::Table, Tags::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order to avoid foreign key constraints
        manager
            .drop_table(Table::drop().table(RecurringTransactionsTags::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RecurringTransactions::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(OneOffTransactionsTags::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(OneOffTransactions::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ManualAccountStates::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Alias::new("accounts_allowed_users")).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AccountsTags::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Accounts::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Tags::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        Ok(())
    }
}

// Define identifiers for all tables

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
}

#[derive(DeriveIden)]
enum Tags {
    Table,
    Id,
    Name,
    Description,
    ParentId,
    LedgerName,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
    Name,
    Description,
    CurrencyCode,
    OwnerId,
    IncludeInStatistics,
    LedgerName,
}

#[derive(DeriveIden)]
enum AccountsTags {
    Table,
    AccountId,
    TagId,
}

#[derive(DeriveIden)]
#[sea_orm(table_name = "accounts_allowed_users")]
enum AccountAllowedUsers {
    Table,
    AccountId,
    UserId,
}

#[derive(DeriveIden)]
enum ManualAccountStates {
    Table,
    Id,
    AccountId,
    Date,
    Amount,
}

#[derive(DeriveIden)]
enum OneOffTransactions {
    Table,
    Id,
    Name,
    Description,
    Amount,
    Date,
    IncludeInStatistics,
    TargetAccountId,
    SourceAccountId,
    LedgerName,
    LinkedImportId,
}

#[derive(DeriveIden)]
enum OneOffTransactionsTags {
    Table,
    TransactionId,
    TagId,
}

#[derive(DeriveIden)]
enum RecurringTransactions {
    Table,
    Id,
    Name,
    Description,
    Amount,
    StartDate,
    EndDate,
    Period,
    IncludeInStatistics,
    TargetAccountId,
    SourceAccountId,
    LedgerName,
}

#[derive(DeriveIden)]
enum RecurringTransactionsTags {
    Table,
    TransactionId,
    TagId,
}
