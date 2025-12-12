use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Create categories table
        manager
            .create_table(
                Table::create()
                    .table(Category::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Category::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Category::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(Category::Description).string())
                    .col(ColumnDef::new(Category::ParentId).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-category-parent")
                            .from(Category::Table, Category::ParentId)
                            .to(Category::Table, Category::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Add category_id to one_off_transactions
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("one_off_transactions"))
                    .add_column(ColumnDef::new(Alias::new("category_id")).integer())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk-one-off-transaction-category")
                            .from_tbl(Alias::new("one_off_transactions"))
                            .from_col(Alias::new("category_id"))
                            .to_tbl(Category::Table)
                            .to_col(Category::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. Add category_id to recurring_transactions
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_transactions"))
                    .add_column(ColumnDef::new(Alias::new("category_id")).integer())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk-recurring-transaction-category")
                            .from_tbl(Alias::new("recurring_transactions"))
                            .from_col(Alias::new("category_id"))
                            .to_tbl(Category::Table)
                            .to_col(Category::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 4. Add category_id to imported_transactions
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("imported_transactions"))
                    .add_column(ColumnDef::new(Alias::new("category_id")).integer())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk-imported-transaction-category")
                            .from_tbl(Alias::new("imported_transactions"))
                            .from_col(Alias::new("category_id"))
                            .to_tbl(Category::Table)
                            .to_col(Category::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 5. Add category_id to recurring_transaction_instances
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_transaction_instances"))
                    .add_column(ColumnDef::new(Alias::new("category_id")).integer())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk-recurring-instance-category")
                            .from_tbl(Alias::new("recurring_transaction_instances"))
                            .from_col(Alias::new("category_id"))
                            .to_tbl(Category::Table)
                            .to_col(Category::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop columns and table in reverse order
        
        // 1. Drop category_id from recurring_transaction_instances
        // Note: SQLite might not support dropping foreign keys easily without table recreation,
        // but SeaORM Manager handles simple drop_column. FKs usually dropped with table or explicitly.
        // For simplicity in SQLite, we just drop columns. FKs might linger or need specific handling if strictly enforced.
        
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_transaction_instances"))
                    .drop_column(Alias::new("category_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("imported_transactions"))
                    .drop_column(Alias::new("category_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_transactions"))
                    .drop_column(Alias::new("category_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("one_off_transactions"))
                    .drop_column(Alias::new("category_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Category::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Category {
    #[sea_orm(iden = "categories")]
    Table,
    Id,
    Name,
    Description,
    ParentId,
}
