pub mod models;
use sqlx::sqlite;

pub type DB = sqlx::Pool<sqlx::Sqlite>;

pub async fn init_db(filename: &str) -> Result<DB, sqlx::Error> {
    sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlite::SqliteConnectOptions::new()
                .filename(filename)
                .create_if_missing(true),
        )
        .await
}

pub async fn migrate_db(pool: &DB) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
