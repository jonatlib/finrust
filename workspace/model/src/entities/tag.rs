use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, QuerySelect, RelationTrait};

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
    #[sea_orm(belongs_to = "Entity", from = "Column::ParentId", to = "Column::Id")]
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

impl Model {
    /// Expands a tag to include all its parent tags up to the root.
    /// Returns an ordered Vec<Tag> from the current tag to the root.
    pub async fn expand(&self, db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
        let mut tags = vec![self.clone()];
        let mut current_tag = self.clone();

        // Traverse up the hierarchy until we reach the root (parent_id is None)
        while let Some(parent_id) = current_tag.parent_id {
            match Entity::find_by_id(parent_id).one(db).await? {
                Some(parent_tag) => {
                    tags.push(parent_tag.clone());
                    current_tag = parent_tag;
                }
                None => break, // Parent not found, stop traversal
            }
        }

        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, Schema, DbBackend, Set, Statement};
    use sea_orm::sea_query::SqliteQueryBuilder;

    async fn setup_test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();

        // Create the tags table
        let schema = Schema::new(DbBackend::Sqlite);
        let stmt = schema.create_table_from_entity(Entity);
        let statement = Statement::from_string(DbBackend::Sqlite, stmt.to_string(SqliteQueryBuilder));
        db.execute(statement).await.unwrap();

        db
    }

    async fn create_test_tag(db: &DatabaseConnection, id: i32, name: &str, description: Option<&str>, parent_id: Option<i32>) -> Model {
        let tag = ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            description: Set(description.map(|s| s.to_string())),
            parent_id: Set(parent_id),
            ledger_name: Set(None),
        };

        tag.insert(db).await.unwrap()
    }

    #[tokio::test]
    async fn test_expand_root_tag() {
        let db = setup_test_db().await;

        // Create a root tag (no parent)
        let root_tag = create_test_tag(&db, 1, "Expenses", Some("Root expenses category"), None).await;

        // Expand should return only the tag itself
        let expanded = root_tag.expand(&db).await.unwrap();

        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].id, 1);
        assert_eq!(expanded[0].name, "Expenses");
        assert_eq!(expanded[0].parent_id, None);
    }

    #[tokio::test]
    async fn test_expand_single_level_hierarchy() {
        let db = setup_test_db().await;

        // Create parent tag
        let parent_tag = create_test_tag(&db, 1, "Expenses", Some("Root expenses category"), None).await;

        // Create child tag
        let child_tag = create_test_tag(&db, 2, "Groceries", Some("Food and groceries"), Some(1)).await;

        // Expand child tag should return [child, parent]
        let expanded = child_tag.expand(&db).await.unwrap();

        assert_eq!(expanded.len(), 2);

        // First should be the child tag itself
        assert_eq!(expanded[0].id, 2);
        assert_eq!(expanded[0].name, "Groceries");
        assert_eq!(expanded[0].parent_id, Some(1));

        // Second should be the parent tag
        assert_eq!(expanded[1].id, 1);
        assert_eq!(expanded[1].name, "Expenses");
        assert_eq!(expanded[1].parent_id, None);
    }

    #[tokio::test]
    async fn test_expand_multi_level_hierarchy() {
        let db = setup_test_db().await;

        // Create a 3-level hierarchy: Root -> Category -> Subcategory
        let root_tag = create_test_tag(&db, 1, "Expenses", Some("Root expenses category"), None).await;
        let category_tag = create_test_tag(&db, 2, "Food", Some("Food expenses"), Some(1)).await;
        let subcategory_tag = create_test_tag(&db, 3, "Groceries", Some("Grocery shopping"), Some(2)).await;

        // Expand the deepest tag should return [subcategory, category, root]
        let expanded = subcategory_tag.expand(&db).await.unwrap();

        assert_eq!(expanded.len(), 3);

        // First should be the subcategory tag itself
        assert_eq!(expanded[0].id, 3);
        assert_eq!(expanded[0].name, "Groceries");
        assert_eq!(expanded[0].parent_id, Some(2));

        // Second should be the category tag
        assert_eq!(expanded[1].id, 2);
        assert_eq!(expanded[1].name, "Food");
        assert_eq!(expanded[1].parent_id, Some(1));

        // Third should be the root tag
        assert_eq!(expanded[2].id, 1);
        assert_eq!(expanded[2].name, "Expenses");
        assert_eq!(expanded[2].parent_id, None);
    }

    #[tokio::test]
    async fn test_expand_broken_hierarchy() {
        let db = setup_test_db().await;

        // Disable foreign key constraints for this test
        let statement = Statement::from_string(DbBackend::Sqlite, "PRAGMA foreign_keys = OFF".to_string());
        db.execute(statement).await.unwrap();

        // Create a tag that references a non-existent parent
        let orphan_tag = create_test_tag(&db, 1, "Orphan", Some("Tag with missing parent"), Some(999)).await;

        // Re-enable foreign key constraints
        let statement = Statement::from_string(DbBackend::Sqlite, "PRAGMA foreign_keys = ON".to_string());
        db.execute(statement).await.unwrap();

        // Expand should handle the missing parent gracefully and return only the tag itself
        let expanded = orphan_tag.expand(&db).await.unwrap();

        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].id, 1);
        assert_eq!(expanded[0].name, "Orphan");
        assert_eq!(expanded[0].parent_id, Some(999));
    }

    #[tokio::test]
    async fn test_expand_complex_hierarchy() {
        let db = setup_test_db().await;

        // Create a 4-level hierarchy: Root -> Level1 -> Level2 -> Level3
        let root = create_test_tag(&db, 1, "Root", Some("Root category"), None).await;
        let level1 = create_test_tag(&db, 2, "Level1", Some("First level"), Some(1)).await;
        let level2 = create_test_tag(&db, 3, "Level2", Some("Second level"), Some(2)).await;
        let level3 = create_test_tag(&db, 4, "Level3", Some("Third level"), Some(3)).await;

        // Test expanding from different levels

        // Expand level1 should return [level1, root]
        let expanded_level1 = level1.expand(&db).await.unwrap();
        assert_eq!(expanded_level1.len(), 2);
        assert_eq!(expanded_level1[0].id, 2);
        assert_eq!(expanded_level1[1].id, 1);

        // Expand level2 should return [level2, level1, root]
        let expanded_level2 = level2.expand(&db).await.unwrap();
        assert_eq!(expanded_level2.len(), 3);
        assert_eq!(expanded_level2[0].id, 3);
        assert_eq!(expanded_level2[1].id, 2);
        assert_eq!(expanded_level2[2].id, 1);

        // Expand level3 should return [level3, level2, level1, root]
        let expanded_level3 = level3.expand(&db).await.unwrap();
        assert_eq!(expanded_level3.len(), 4);
        assert_eq!(expanded_level3[0].id, 4);
        assert_eq!(expanded_level3[1].id, 3);
        assert_eq!(expanded_level3[2].id, 2);
        assert_eq!(expanded_level3[3].id, 1);
    }
}
