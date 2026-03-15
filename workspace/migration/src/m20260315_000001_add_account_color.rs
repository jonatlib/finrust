use sea_orm::Statement;
use sea_orm_migration::prelude::*;

const ACCOUNT_COLORS: &[&str] = &[
    "#3b82f6", "#22c55e", "#a855f7", "#ef4444", "#f59e0b", "#06b6d4",
    "#ec4899", "#84cc16", "#6366f1", "#14b8a6", "#f97316", "#8b5cf6",
];

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("accounts"))
                    .add_column(ColumnDef::new(Alias::new("color")).string_len(7))
                    .to_owned(),
            )
            .await?;

        // Backfill existing accounts with deterministic colors based on id order
        let db = manager.get_connection();
        let rows = db
            .query_all(Statement::from_string(
                manager.get_database_backend(),
                "SELECT id FROM accounts ORDER BY id".to_owned(),
            ))
            .await?;

        for (idx, row) in rows.iter().enumerate() {
            let id: i32 = row.try_get("", "id")?;
            let color = ACCOUNT_COLORS[idx % ACCOUNT_COLORS.len()];
            db.execute(Statement::from_string(
                manager.get_database_backend(),
                format!("UPDATE accounts SET color = '{}' WHERE id = {}", color, id),
            ))
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("accounts"))
                    .drop_column(Alias::new("color"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
