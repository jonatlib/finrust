use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Create scenarios table
        manager
            .create_table(
                Table::create()
                    .table(Scenario::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Scenario::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Scenario::Name).string().not_null())
                    .col(ColumnDef::new(Scenario::Description).string())
                    .col(
                        ColumnDef::new(Scenario::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Scenario::IsActive)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Add scenario_id and is_simulated to one_off_transactions
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("one_off_transactions"))
                    .add_column(ColumnDef::new(Alias::new("scenario_id")).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("one_off_transactions"))
                    .add_column(
                        ColumnDef::new(Alias::new("is_simulated"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Note: SQLite doesn't support adding foreign keys to existing tables
        // The foreign key constraint is defined in the entity model and will be enforced by SeaORM

        // 3. Add scenario_id and is_simulated to recurring_transactions
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_transactions"))
                    .add_column(ColumnDef::new(Alias::new("scenario_id")).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_transactions"))
                    .add_column(
                        ColumnDef::new(Alias::new("is_simulated"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // 4. Add scenario_id and is_simulated to recurring_incomes
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_incomes"))
                    .add_column(ColumnDef::new(Alias::new("scenario_id")).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_incomes"))
                    .add_column(
                        ColumnDef::new(Alias::new("is_simulated"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop foreign keys and columns in reverse order

        // 1. Drop from recurring_incomes
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_incomes"))
                    .drop_column(Alias::new("is_simulated"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_incomes"))
                    .drop_column(Alias::new("scenario_id"))
                    .to_owned(),
            )
            .await?;

        // 2. Drop from recurring_transactions
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_transactions"))
                    .drop_column(Alias::new("is_simulated"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("recurring_transactions"))
                    .drop_column(Alias::new("scenario_id"))
                    .to_owned(),
            )
            .await?;

        // 3. Drop from one_off_transactions
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("one_off_transactions"))
                    .drop_column(Alias::new("is_simulated"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("one_off_transactions"))
                    .drop_column(Alias::new("scenario_id"))
                    .to_owned(),
            )
            .await?;

        // 4. Drop scenarios table
        manager
            .drop_table(Table::drop().table(Scenario::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Scenario {
    #[sea_orm(iden = "scenarios")]
    Table,
    Id,
    Name,
    Description,
    CreatedAt,
    IsActive,
}
