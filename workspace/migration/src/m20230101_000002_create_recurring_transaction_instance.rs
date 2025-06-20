use crate::entity_iden::EntityIden;
use model::entities::prelude::*;
use model::entities::{
    imported_transaction, recurring_transaction, recurring_transaction_instance,
};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create recurring_transaction_instances table
        manager
            .create_table(
                Table::create()
                    .table(RecurringTransactionInstance::table())
                    .if_not_exists()
                    .col(pk_auto(RecurringTransactionInstance::column(
                        recurring_transaction_instance::Column::Id,
                    )))
                    .col(integer(RecurringTransactionInstance::column(
                        recurring_transaction_instance::Column::RecurringTransactionId,
                    )))
                    .col(string(RecurringTransactionInstance::column(
                        recurring_transaction_instance::Column::Status,
                    )).string_len(15))
                    .col(date(RecurringTransactionInstance::column(
                        recurring_transaction_instance::Column::DueDate,
                    )))
                    .col(
                        decimal(RecurringTransactionInstance::column(
                            recurring_transaction_instance::Column::ExpectedAmount,
                        ))
                        .decimal_len(16, 4),
                    )
                    .col(date_null(RecurringTransactionInstance::column(
                        recurring_transaction_instance::Column::PaidDate,
                    )))
                    .col(
                        decimal_null(RecurringTransactionInstance::column(
                            recurring_transaction_instance::Column::PaidAmount,
                        ))
                        .decimal_len(16, 4),
                    )
                    .col(integer_null(RecurringTransactionInstance::column(
                        recurring_transaction_instance::Column::ReconciledImportedTransactionId,
                    )))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_transaction_instances_recurring_transaction")
                            .from(
                                RecurringTransactionInstance::table(),
                                RecurringTransactionInstance::column(
                                    recurring_transaction_instance::Column::RecurringTransactionId,
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
                            .name("fk_recurring_transaction_instances_imported_transaction")
                            .from(
                                RecurringTransactionInstance::table(),
                                RecurringTransactionInstance::column(
                                    recurring_transaction_instance::Column::ReconciledImportedTransactionId,
                                ),
                            )
                            .to(
                                ImportedTransaction::table(),
                                ImportedTransaction::column(imported_transaction::Column::Id),
                            )
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the recurring_transaction_instances table
        manager
            .drop_table(
                Table::drop()
                    .table(RecurringTransactionInstance::table())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
