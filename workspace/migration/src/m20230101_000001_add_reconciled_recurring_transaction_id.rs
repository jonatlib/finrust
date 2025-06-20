use crate::entity_iden::EntityIden;
use model::entities::prelude::*;
use model::entities::{one_off_transaction, recurring_transaction};
use sea_orm_migration::prelude::*;

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

        // Note: We're not adding a foreign key constraint for simplicity
        // This can be added later if needed

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove the column
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
