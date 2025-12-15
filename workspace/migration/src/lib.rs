pub use sea_orm_migration::prelude::*;

pub mod entity_iden;
mod m20220101_000001_create_table;
mod m20230101_000002_create_recurring_transaction_instance;
mod m20230101_000003_update_imported_transaction_type;
mod m20250101_000001_add_account_kind;
mod m20251212_000001_add_categories;
mod m20251215_000001_add_scenarios;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230101_000002_create_recurring_transaction_instance::Migration),
            Box::new(m20230101_000003_update_imported_transaction_type::Migration),
            Box::new(m20250101_000001_add_account_kind::Migration),
            Box::new(m20251212_000001_add_categories::Migration),
            Box::new(m20251215_000001_add_scenarios::Migration),
        ]
    }
}
