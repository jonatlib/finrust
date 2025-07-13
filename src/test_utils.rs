#[cfg(test)]
pub mod test_utils {
    use crate::router::create_router;
    use crate::schemas::AppState;
    use axum::Router;
    use moka::future::Cache;
    use sea_orm::{Database, DatabaseConnection};
    use migration::{Migrator, MigratorTrait};

    /// Create an in-memory SQLite database for testing
    pub async fn setup_test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to in-memory database");

        // Run migrations
        Migrator::up(&db, None)
            .await
            .expect("Failed to run migrations");

        db
    }

    /// Create AppState for testing
    pub async fn setup_test_app_state() -> AppState {
        let db = setup_test_db().await;
        let cache = Cache::new(100);

        AppState { db, cache }
    }

    /// Create axum app for testing
    pub async fn setup_test_app() -> Router {
        // Initialize tracing for tests
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .try_init();

        let state = setup_test_app_state().await;
        println!("Test database setup complete");
        let router = create_router(state);
        println!("Test router created");
        router
    }
}
