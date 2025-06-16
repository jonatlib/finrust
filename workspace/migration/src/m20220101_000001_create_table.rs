use crate::entity_iden::EntityIden;
use model::entities::prelude::*;
use model::entities::{
    account, account_allowed_user, account_tag, imported_transaction, manual_account_state, one_off_transaction,
    one_off_transaction_tag, recurring_income, recurring_income_tag, recurring_transaction, 
    recurring_transaction_tag, tag, user,
};
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
                    .table(User::table())
                    .if_not_exists()
                    .col(pk_auto(User::column(user::Column::Id)))
                    .col(string(User::column(user::Column::Username)).unique_key())
                    .to_owned(),
            )
            .await?;

        // Create tags table
        manager
            .create_table(
                Table::create()
                    .table(Tag::table())
                    .if_not_exists()
                    .col(pk_auto(Tag::column(tag::Column::Id)))
                    .col(string(Tag::column(tag::Column::Name)).unique_key())
                    .col(string_null(Tag::column(tag::Column::Description)))
                    .col(integer_null(Tag::column(tag::Column::ParentId)))
                    .col(string_null(Tag::column(tag::Column::LedgerName)))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tag_parent")
                            .from(Tag::table(), Tag::column(tag::Column::ParentId))
                            .to(Tag::table(), Tag::column(tag::Column::Id))
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
                    .table(Account::table())
                    .if_not_exists()
                    .col(pk_auto(Account::column(account::Column::Id)))
                    .col(string(Account::column(account::Column::Name)))
                    .col(string_null(Account::column(account::Column::Description)))
                    .col(string(Account::column(account::Column::CurrencyCode)))
                    .col(integer(Account::column(account::Column::OwnerId)))
                    .col(
                        boolean(Account::column(account::Column::IncludeInStatistics))
                            .default(true),
                    )
                    .col(string_null(Account::column(account::Column::LedgerName)))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_account_owner")
                            .from(Account::table(), Account::column(account::Column::OwnerId))
                            .to(User::table(), User::column(user::Column::Id))
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
                    .table(AccountTag::table())
                    .if_not_exists()
                    .col(integer(AccountTag::column(account_tag::Column::AccountId)))
                    .col(integer(AccountTag::column(account_tag::Column::TagId)))
                    .primary_key(
                        Index::create()
                            .name("pk_accounts_tags")
                            .col(AccountTag::column(account_tag::Column::AccountId))
                            .col(AccountTag::column(account_tag::Column::TagId)),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_tags_account")
                            .from(
                                AccountTag::table(),
                                AccountTag::column(account_tag::Column::AccountId),
                            )
                            .to(Account::table(), Account::column(account::Column::Id))
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_tags_tag")
                            .from(
                                AccountTag::table(),
                                AccountTag::column(account_tag::Column::TagId),
                            )
                            .to(Tag::table(), Tag::column(tag::Column::Id))
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
                    .table(AccountAllowedUser::table())
                    .if_not_exists()
                    .col(integer(AccountAllowedUser::column(
                        account_allowed_user::Column::AccountId,
                    )))
                    .col(integer(AccountAllowedUser::column(
                        account_allowed_user::Column::UserId,
                    )))
                    .primary_key(
                        Index::create()
                            .name("pk_accounts_allowed_users")
                            .col(AccountAllowedUser::column(
                                account_allowed_user::Column::AccountId,
                            ))
                            .col(AccountAllowedUser::column(
                                account_allowed_user::Column::UserId,
                            )),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_allowed_users_account")
                            .from(
                                AccountAllowedUser::table(),
                                AccountAllowedUser::column(account_allowed_user::Column::AccountId),
                            )
                            .to(Account::table(), Account::column(account::Column::Id))
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_allowed_users_user")
                            .from(
                                AccountAllowedUser::table(),
                                AccountAllowedUser::column(account_allowed_user::Column::UserId),
                            )
                            .to(User::table(), User::column(user::Column::Id))
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
                    .table(ManualAccountState::table())
                    .if_not_exists()
                    .col(pk_auto(ManualAccountState::column(
                        manual_account_state::Column::Id,
                    )))
                    .col(integer(ManualAccountState::column(
                        manual_account_state::Column::AccountId,
                    )))
                    .col(date(ManualAccountState::column(
                        manual_account_state::Column::Date,
                    )))
                    .col(
                        decimal(ManualAccountState::column(
                            manual_account_state::Column::Amount,
                        ))
                        .decimal_len(16, 4),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_manual_account_states_account")
                            .from(
                                ManualAccountState::table(),
                                ManualAccountState::column(manual_account_state::Column::AccountId),
                            )
                            .to(Account::table(), Account::column(account::Column::Id))
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
                    .table(OneOffTransaction::table())
                    .if_not_exists()
                    .col(pk_auto(OneOffTransaction::column(
                        one_off_transaction::Column::Id,
                    )))
                    .col(string(OneOffTransaction::column(
                        one_off_transaction::Column::Name,
                    )))
                    .col(string_null(OneOffTransaction::column(
                        one_off_transaction::Column::Description,
                    )))
                    .col(
                        decimal(OneOffTransaction::column(
                            one_off_transaction::Column::Amount,
                        ))
                        .decimal_len(16, 4),
                    )
                    .col(date(OneOffTransaction::column(
                        one_off_transaction::Column::Date,
                    )))
                    .col(
                        boolean(OneOffTransaction::column(
                            one_off_transaction::Column::IncludeInStatistics,
                        ))
                        .default(true),
                    )
                    .col(integer(OneOffTransaction::column(
                        one_off_transaction::Column::TargetAccountId,
                    )))
                    .col(integer_null(OneOffTransaction::column(
                        one_off_transaction::Column::SourceAccountId,
                    )))
                    .col(string_null(OneOffTransaction::column(
                        one_off_transaction::Column::LedgerName,
                    )))
                    .col(string_null(OneOffTransaction::column(
                        one_off_transaction::Column::LinkedImportId,
                    )))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_off_transactions_target_account")
                            .from(
                                OneOffTransaction::table(),
                                OneOffTransaction::column(
                                    one_off_transaction::Column::TargetAccountId,
                                ),
                            )
                            .to(Account::table(), Account::column(account::Column::Id))
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_off_transactions_source_account")
                            .from(
                                OneOffTransaction::table(),
                                OneOffTransaction::column(
                                    one_off_transaction::Column::SourceAccountId,
                                ),
                            )
                            .to(Account::table(), Account::column(account::Column::Id))
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
                    .table(OneOffTransactionTag::table())
                    .if_not_exists()
                    .col(integer(OneOffTransactionTag::column(
                        one_off_transaction_tag::Column::TransactionId,
                    )))
                    .col(integer(OneOffTransactionTag::column(
                        one_off_transaction_tag::Column::TagId,
                    )))
                    .primary_key(
                        Index::create()
                            .name("pk_one_off_transactions_tags")
                            .col(OneOffTransactionTag::column(
                                one_off_transaction_tag::Column::TransactionId,
                            ))
                            .col(OneOffTransactionTag::column(
                                one_off_transaction_tag::Column::TagId,
                            )),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_off_transactions_tags_transaction")
                            .from(
                                OneOffTransactionTag::table(),
                                OneOffTransactionTag::column(
                                    one_off_transaction_tag::Column::TransactionId,
                                ),
                            )
                            .to(
                                OneOffTransaction::table(),
                                OneOffTransaction::column(one_off_transaction::Column::Id),
                            )
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_off_transactions_tags_tag")
                            .from(
                                OneOffTransactionTag::table(),
                                OneOffTransactionTag::column(
                                    one_off_transaction_tag::Column::TagId,
                                ),
                            )
                            .to(Tag::table(), Tag::column(tag::Column::Id))
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
                    .table(RecurringTransaction::table())
                    .if_not_exists()
                    .col(pk_auto(RecurringTransaction::column(
                        recurring_transaction::Column::Id,
                    )))
                    .col(string(RecurringTransaction::column(
                        recurring_transaction::Column::Name,
                    )))
                    .col(string_null(RecurringTransaction::column(
                        recurring_transaction::Column::Description,
                    )))
                    .col(
                        decimal(RecurringTransaction::column(
                            recurring_transaction::Column::Amount,
                        ))
                        .decimal_len(16, 4),
                    )
                    .col(date(RecurringTransaction::column(
                        recurring_transaction::Column::StartDate,
                    )))
                    .col(date_null(RecurringTransaction::column(
                        recurring_transaction::Column::EndDate,
                    )))
                    .col(
                        string(RecurringTransaction::column(
                            recurring_transaction::Column::Period,
                        ))
                        .string_len(1),
                    )
                    .col(
                        boolean(RecurringTransaction::column(
                            recurring_transaction::Column::IncludeInStatistics,
                        ))
                        .default(true),
                    )
                    .col(integer(RecurringTransaction::column(
                        recurring_transaction::Column::TargetAccountId,
                    )))
                    .col(integer_null(RecurringTransaction::column(
                        recurring_transaction::Column::SourceAccountId,
                    )))
                    .col(string_null(RecurringTransaction::column(
                        recurring_transaction::Column::LedgerName,
                    )))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_transactions_target_account")
                            .from(
                                RecurringTransaction::table(),
                                RecurringTransaction::column(
                                    recurring_transaction::Column::TargetAccountId,
                                ),
                            )
                            .to(Account::table(), Account::column(account::Column::Id))
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_transactions_source_account")
                            .from(
                                RecurringTransaction::table(),
                                RecurringTransaction::column(
                                    recurring_transaction::Column::SourceAccountId,
                                ),
                            )
                            .to(Account::table(), Account::column(account::Column::Id))
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
                    .table(RecurringTransactionTag::table())
                    .if_not_exists()
                    .col(integer(RecurringTransactionTag::column(
                        recurring_transaction_tag::Column::TransactionId,
                    )))
                    .col(integer(RecurringTransactionTag::column(
                        recurring_transaction_tag::Column::TagId,
                    )))
                    .primary_key(
                        Index::create()
                            .name("pk_recurring_transactions_tags")
                            .col(RecurringTransactionTag::column(
                                recurring_transaction_tag::Column::TransactionId,
                            ))
                            .col(RecurringTransactionTag::column(
                                recurring_transaction_tag::Column::TagId,
                            )),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_transactions_tags_transaction")
                            .from(
                                RecurringTransactionTag::table(),
                                RecurringTransactionTag::column(
                                    recurring_transaction_tag::Column::TransactionId,
                                ),
                            )
                            .to(
                                RecurringTransaction::table(),
                                RecurringTransaction::column(recurring_transaction::Column::Id),
                            )
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_transactions_tags_tag")
                            .from(
                                RecurringTransactionTag::table(),
                                RecurringTransactionTag::column(
                                    recurring_transaction_tag::Column::TagId,
                                ),
                            )
                            .to(Tag::table(), Tag::column(tag::Column::Id))
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create imported_transactions table
        manager
            .create_table(
                Table::create()
                    .table(ImportedTransaction::table())
                    .if_not_exists()
                    .col(pk_auto(ImportedTransaction::column(
                        imported_transaction::Column::Id,
                    )))
                    .col(integer(ImportedTransaction::column(
                        imported_transaction::Column::AccountId,
                    )))
                    .col(date(ImportedTransaction::column(
                        imported_transaction::Column::Date,
                    )))
                    .col(string(ImportedTransaction::column(
                        imported_transaction::Column::Description,
                    )))
                    .col(
                        decimal(ImportedTransaction::column(
                            imported_transaction::Column::Amount,
                        ))
                        .decimal_len(16, 4),
                    )
                    .col(string(ImportedTransaction::column(
                        imported_transaction::Column::ImportHash,
                    )).unique_key())
                    .col(json_binary_null(ImportedTransaction::column(
                        imported_transaction::Column::RawData,
                    )))
                    .col(integer_null(ImportedTransaction::column(
                        imported_transaction::Column::ReconciledOneOffTransactionId,
                    )))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_imported_transactions_account")
                            .from(
                                ImportedTransaction::table(),
                                ImportedTransaction::column(
                                    imported_transaction::Column::AccountId,
                                ),
                            )
                            .to(Account::table(), Account::column(account::Column::Id))
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_imported_transactions_reconciled_transaction")
                            .from(
                                ImportedTransaction::table(),
                                ImportedTransaction::column(
                                    imported_transaction::Column::ReconciledOneOffTransactionId,
                                ),
                            )
                            .to(
                                OneOffTransaction::table(),
                                OneOffTransaction::column(one_off_transaction::Column::Id),
                            )
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create recurring_incomes table
        manager
            .create_table(
                Table::create()
                    .table(RecurringIncome::table())
                    .if_not_exists()
                    .col(pk_auto(RecurringIncome::column(
                        recurring_income::Column::Id,
                    )))
                    .col(string(RecurringIncome::column(
                        recurring_income::Column::Name,
                    )))
                    .col(string_null(RecurringIncome::column(
                        recurring_income::Column::Description,
                    )))
                    .col(
                        decimal(RecurringIncome::column(
                            recurring_income::Column::Amount,
                        ))
                        .decimal_len(16, 4),
                    )
                    .col(date(RecurringIncome::column(
                        recurring_income::Column::StartDate,
                    )))
                    .col(date_null(RecurringIncome::column(
                        recurring_income::Column::EndDate,
                    )))
                    .col(
                        string(RecurringIncome::column(
                            recurring_income::Column::Period,
                        ))
                        .string_len(1),
                    )
                    .col(
                        boolean(RecurringIncome::column(
                            recurring_income::Column::IncludeInStatistics,
                        ))
                        .default(true),
                    )
                    .col(integer(RecurringIncome::column(
                        recurring_income::Column::TargetAccountId,
                    )))
                    .col(string_null(RecurringIncome::column(
                        recurring_income::Column::SourceName,
                    )))
                    .col(string_null(RecurringIncome::column(
                        recurring_income::Column::LedgerName,
                    )))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_incomes_target_account")
                            .from(
                                RecurringIncome::table(),
                                RecurringIncome::column(
                                    recurring_income::Column::TargetAccountId,
                                ),
                            )
                            .to(Account::table(), Account::column(account::Column::Id))
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create recurring_incomes_tags table (join table)
        manager
            .create_table(
                Table::create()
                    .table(RecurringIncomeTag::table())
                    .if_not_exists()
                    .col(integer(RecurringIncomeTag::column(
                        recurring_income_tag::Column::IncomeId,
                    )))
                    .col(integer(RecurringIncomeTag::column(
                        recurring_income_tag::Column::TagId,
                    )))
                    .primary_key(
                        Index::create()
                            .name("pk_recurring_incomes_tags")
                            .col(RecurringIncomeTag::column(
                                recurring_income_tag::Column::IncomeId,
                            ))
                            .col(RecurringIncomeTag::column(
                                recurring_income_tag::Column::TagId,
                            )),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_incomes_tags_income")
                            .from(
                                RecurringIncomeTag::table(),
                                RecurringIncomeTag::column(
                                    recurring_income_tag::Column::IncomeId,
                                ),
                            )
                            .to(
                                RecurringIncome::table(),
                                RecurringIncome::column(recurring_income::Column::Id),
                            )
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_incomes_tags_tag")
                            .from(
                                RecurringIncomeTag::table(),
                                RecurringIncomeTag::column(
                                    recurring_income_tag::Column::TagId,
                                ),
                            )
                            .to(Tag::table(), Tag::column(tag::Column::Id))
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
            .drop_table(
                Table::drop()
                    .table(RecurringIncomeTag::table())
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(RecurringIncome::table())
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(RecurringTransactionTag::table())
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(RecurringTransaction::table())
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(OneOffTransactionTag::table())
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(OneOffTransaction::table()).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ImportedTransaction::table()).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ManualAccountState::table()).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AccountAllowedUser::table()).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AccountTag::table()).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Account::table()).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Tag::table()).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(User::table()).to_owned())
            .await?;

        Ok(())
    }
}
