#[cfg(test)]
pub mod test_utils {
    use crate::router::create_router;
    use crate::schemas::AppState;
    use axum::Router;
    use migration::{Migrator, MigratorTrait};
    use moka::future::Cache;
    use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

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

        // Create test users for the tests to reference
        let test_user1 = model::entities::user::ActiveModel {
            username: Set("test_user1".to_string()),
            ..Default::default()
        };

        let test_user2 = model::entities::user::ActiveModel {
            username: Set("test_user2".to_string()),
            ..Default::default()
        };

        test_user1.insert(&db).await.expect("Failed to create test user 1");
        test_user2.insert(&db).await.expect("Failed to create test user 2");

        let cache = Cache::new(100);

        AppState { db, cache }
    }

    /// Initialize tracing for tests with output to STDERR.
    ///
    /// This function sets up a tracing subscriber that outputs logs to STDERR,
    /// which is useful for debugging tests. The log level is determined by the
    /// RUST_LOG environment variable, defaulting to WARN if not set.
    ///
    /// # Returns
    ///
    /// A guard that will clean up the subscriber when dropped.
    fn init_test_tracing() -> tracing::subscriber::DefaultGuard {
        // Get log level from environment variable or default to WARN
        let log_level = std::env::var("RUST_LOG")
            .ok()
            .and_then(|level| match level.to_uppercase().as_str() {
                "ERROR" => Some(Level::ERROR),
                "WARN" => Some(Level::WARN),
                "INFO" => Some(Level::INFO),
                "DEBUG" => Some(Level::DEBUG),
                "TRACE" => Some(Level::TRACE),
                _ => None,
            })
            .unwrap_or(Level::WARN);

        let subscriber = FmtSubscriber::builder()
            .with_max_level(log_level)
            .with_writer(std::io::stderr) // Output to stderr, which is captured by tests
            .finish();
        tracing::subscriber::set_default(subscriber)
    }

    /// Create axum app for testing
    pub async fn setup_test_app() -> Router {
        // Initialize tracing for tests
        let _ = init_test_tracing();

        let state = setup_test_app_state().await;
        println!("Test database setup complete");
        let router = create_router(state);
        println!("Test router created");
        router
    }
}
