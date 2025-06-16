use sea_orm::entity::prelude::*;

/// Represents a tag that can be applied to accounts or transactions.
/// Tags can be hierarchical (e.g., "Expenses" -> "Groceries").
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    /// A description of what the tag is for.
    pub description: Option<String>,
    /// Self-referencing foreign key for hierarchical tags.
    pub parent_id: Option<i32>,
    /// The name to use when exporting to Ledger CLI format.
    /// Can be a template string.
    pub ledger_name: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Defines the self-referencing relationship for parent tag.
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentId",
        to = "Column::Id"
    )]
    Parent,
}

// Implement Related trait for self-referencing relationship
impl Related<Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Parent.def()
    }

    fn via() -> Option<RelationDef> {
        None
    }
}

impl ActiveModelBehavior for ActiveModel {}
