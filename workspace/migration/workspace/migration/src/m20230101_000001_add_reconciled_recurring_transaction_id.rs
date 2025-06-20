use crate::entity_iden::EntityIden;
use model::entities::{one_off_transaction, recurring_transaction};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add reconciled_recurring_transaction_id column to one_off_transactions table
        manager
            .alter_table(
                Table::alter()
                    .table(OneOffTransaction::table())
                    .add_column(
                        ColumnDef::new(OneOffTransaction::column(
                            one_off_transaction::Column::ReconciledRecurringTransactionId,
                        ))
                        .integer()
                        .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint
        manager
            .alter_table(
                Table::alter()
                    .table(OneOffTransaction::table())
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("fk_one_off_transactions_reconciled_recurring_transaction")
                            .from(
                                OneOffTransaction::table(),
                                OneOffTransaction::column(
                                    one_off_transaction::Column::ReconciledRecurringTransactionId,
                                ),
                            )
                            .to(
                                RecurringTransaction::table(),
                                RecurringTransaction::column(recurring_transaction::Column::Id),
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
        // Remove foreign key constraint first
        manager
            .alter_table(
                Table::alter()
                    .table(OneOffTransaction::table())
                    .drop_foreign_key("fk_one_off_transactions_reconciled_recurring_transaction")
                    .to_owned(),
            )
            .await?;

        // Then remove the column
        manager
            .alter_table(
                Table::alter()
                    .table(OneOffTransaction::table())
                    .drop_column(OneOffTransaction::column(
                        one_off_transaction::Column::ReconciledRecurringTransactionId,
                    ))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}