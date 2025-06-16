use model::entities::prelude::*;
use sea_orm::entity::prelude::*;
use sea_orm::Iden;

/// A trait for converting an entity into an identifier that can be used in migrations.
pub trait EntityIden: EntityTrait {
    /// Get the table identifier for this entity.
    fn table() -> TableIden {
        TableIden(Self::default().table_name().to_string())
    }

    /// Get a column identifier for this entity.
    fn column<C: ColumnTrait + Iden>(column: C) -> ColumnIden {
        let mut s = String::new();
        column.unquoted(&mut s);
        ColumnIden(s)
    }
}

/// Implement EntityIden for all entity types.
impl EntityIden for User {}
impl EntityIden for Tag {}
impl EntityIden for Account {}
impl EntityIden for AccountTag {}
impl EntityIden for AccountAllowedUser {}
impl EntityIden for ManualAccountState {}
impl EntityIden for OneOffTransaction {}
impl EntityIden for OneOffTransactionTag {}
impl EntityIden for RecurringTransaction {}
impl EntityIden for RecurringTransactionTag {}

/// A wrapper for table identifiers.
#[derive(Debug, Clone)]
pub struct TableIden(String);

impl Iden for TableIden {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        let _ = s.write_str(&self.0);
    }
}

/// A wrapper for column identifiers.
#[derive(Debug, Clone)]
pub struct ColumnIden(String);

impl Iden for ColumnIden {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        let _ = s.write_str(&self.0);
    }
}
