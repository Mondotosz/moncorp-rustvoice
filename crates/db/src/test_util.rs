use sea_orm::DatabaseConnection;

/// A fresh, fully-migrated in-memory SQLite database for repository tests.
pub(crate) async fn test_db() -> DatabaseConnection {
    crate::connection::connect_in_memory_for_tests()
        .await
        .expect("connect in-memory test db")
}
