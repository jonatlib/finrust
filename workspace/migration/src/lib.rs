pub use sea_orm_migration::prelude::*;

pub mod entity_iden;
mod m20220101_000001_create_table;
mod m20230101_000001_add_reconciled_recurring_transaction_id;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230101_000001_add_reconciled_recurring_transaction_id::Migration),
        ]
    }
}
