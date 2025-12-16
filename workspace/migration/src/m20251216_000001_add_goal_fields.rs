use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add target_amount column to accounts table
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("accounts"))
                    .add_column(ColumnDef::new(Alias::new("target_amount")).decimal())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop target_amount column from accounts table
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("accounts"))
                    .drop_column(Alias::new("target_amount"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
