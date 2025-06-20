use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // This migration doesn't need to modify the database schema
        // The RecurringInstance variant is added to the ReconciledTransactionEntityType enum
        // which is stored as a string in the database
        // The database column already supports string values, so no schema change is needed
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // No changes to revert
        Ok(())
    }
}
