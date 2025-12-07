use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("accounts"))
                    .add_column(
                        ColumnDef::new(Alias::new("account_kind"))
                            .string_len(20)
                            .not_null()
                            .default("RealAccount")
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("accounts"))
                    .drop_column(Alias::new("account_kind"))
                    .to_owned(),
            )
            .await
    }
}
