use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, RelationTrait};

/// Represents a category for transactions.
/// Categories are hierarchical (e.g., "Food" -> "Groceries").
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "categories")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    /// A description of what the category is for.
    pub description: Option<String>,
    /// Self-referencing foreign key for hierarchical categories.
    pub parent_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Defines the self-referencing relationship for parent category.
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
    /// Expands a category to include all its parent categories up to the root.
    /// Returns an ordered Vec<Category> from the current category to the root.
    pub async fn expand(&self, db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
        let mut categories = vec![self.clone()];
        let mut current_category = self.clone();

        // Traverse up the hierarchy until we reach the root (parent_id is None)
        while let Some(parent_id) = current_category.parent_id {
            match Entity::find_by_id(parent_id).one(db).await? {
                Some(parent_category) => {
                    categories.push(parent_category.clone());
                    current_category = parent_category;
                }
                None => break, // Parent not found, stop traversal
            }
        }

        Ok(categories)
    }

    /// Gets all direct children of this category.
    pub async fn get_children(&self, db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::ParentId.eq(self.id))
            .all(db)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::sea_query::SqliteQueryBuilder;
    use sea_orm::{Database, DbBackend, Schema, Set, Statement};

    async fn setup_test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();

        // Create the categories table
        let schema = Schema::new(DbBackend::Sqlite);
        let stmt = schema.create_table_from_entity(Entity);
        let statement =
            Statement::from_string(DbBackend::Sqlite, stmt.to_string(SqliteQueryBuilder));
        db.execute(statement).await.unwrap();

        db
    }

    async fn create_test_category(
        db: &DatabaseConnection,
        id: i32,
        name: &str,
        description: Option<&str>,
        parent_id: Option<i32>,
    ) -> Model {
        let category = ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            description: Set(description.map(|s| s.to_string())),
            parent_id: Set(parent_id),
        };

        category.insert(db).await.unwrap()
    }

    #[tokio::test]
    async fn test_expand_root_category() {
        let db = setup_test_db().await;

        // Create a root category (no parent)
        let root_category =
            create_test_category(&db, 1, "Expenses", Some("Root expenses category"), None).await;

        // Expand should return only the category itself
        let expanded = root_category.expand(&db).await.unwrap();

        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].id, 1);
        assert_eq!(expanded[0].name, "Expenses");
        assert_eq!(expanded[0].parent_id, None);
    }

    #[tokio::test]
    async fn test_expand_single_level_hierarchy() {
        let db = setup_test_db().await;

        // Create parent category
        let parent_category =
            create_test_category(&db, 1, "Expenses", Some("Root expenses category"), None).await;

        // Create child category
        let child_category =
            create_test_category(&db, 2, "Groceries", Some("Food and groceries"), Some(1)).await;

        // Expand child category should return [child, parent]
        let expanded = child_category.expand(&db).await.unwrap();

        assert_eq!(expanded.len(), 2);

        // First should be the child category itself
        assert_eq!(expanded[0].id, 2);
        assert_eq!(expanded[0].name, "Groceries");
        assert_eq!(expanded[0].parent_id, Some(1));

        // Second should be the parent category
        assert_eq!(expanded[1].id, 1);
        assert_eq!(expanded[1].name, "Expenses");
        assert_eq!(expanded[1].parent_id, None);
    }

    #[tokio::test]
    async fn test_expand_multi_level_hierarchy() {
        let db = setup_test_db().await;

        // Create a 3-level hierarchy: Root -> Category -> Subcategory
        let root_category =
            create_test_category(&db, 1, "Expenses", Some("Root expenses category"), None).await;
        let category = create_test_category(&db, 2, "Food", Some("Food expenses"), Some(1)).await;
        let subcategory =
            create_test_category(&db, 3, "Groceries", Some("Grocery shopping"), Some(2)).await;

        // Expand the deepest category should return [subcategory, category, root]
        let expanded = subcategory.expand(&db).await.unwrap();

        assert_eq!(expanded.len(), 3);

        // First should be the subcategory itself
        assert_eq!(expanded[0].id, 3);
        assert_eq!(expanded[0].name, "Groceries");
        assert_eq!(expanded[0].parent_id, Some(2));

        // Second should be the category
        assert_eq!(expanded[1].id, 2);
        assert_eq!(expanded[1].name, "Food");
        assert_eq!(expanded[1].parent_id, Some(1));

        // Third should be the root category
        assert_eq!(expanded[2].id, 1);
        assert_eq!(expanded[2].name, "Expenses");
    }

    #[tokio::test]
    async fn test_expand_with_null_parent() {
        let db = setup_test_db().await;

        // Create a category with no parent (NULL parent_id)
        let root_category =
            create_test_category(&db, 1, "Groceries", Some("Food and groceries"), None).await;

        // Expand should only return the category itself since it has no parent
        let expanded = root_category.expand(&db).await.unwrap();

        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].id, 1);
        assert_eq!(expanded[0].name, "Groceries");
        assert_eq!(expanded[0].parent_id, None);
    }

    #[tokio::test]
    async fn test_get_children() {
        let db = setup_test_db().await;

        let root = create_test_category(&db, 1, "Root", None, None).await;
        let child1 = create_test_category(&db, 2, "Child1", None, Some(1)).await;
        let child2 = create_test_category(&db, 3, "Child2", None, Some(1)).await;
        let grandchild = create_test_category(&db, 4, "Grandchild", None, Some(2)).await;

        let children_of_root = root.get_children(&db).await.unwrap();
        assert_eq!(children_of_root.len(), 2);
        assert!(children_of_root.iter().any(|c| c.id == 2));
        assert!(children_of_root.iter().any(|c| c.id == 3));

        let children_of_child1 = child1.get_children(&db).await.unwrap();
        assert_eq!(children_of_child1.len(), 1);
        assert_eq!(children_of_child1[0].id, 4);

        let children_of_child2 = child2.get_children(&db).await.unwrap();
        assert_eq!(children_of_child2.len(), 0);
    }
}
