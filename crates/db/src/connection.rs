use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

use crate::migrator::Migrator;

/// Connect without running migrations — useful for checking migration status.
pub async fn connect_raw(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    ensure_sqlite_dir(database_url)?;
    let mut opts = ConnectOptions::new(database_url);
    opts.max_connections(5).sqlx_logging(false);
    Database::connect(opts).await
}

pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    ensure_sqlite_dir(database_url)?;

    let mut opts = ConnectOptions::new(database_url);
    opts.max_connections(5).sqlx_logging(false);

    let db = Database::connect(opts).await?;
    Migrator::up(&db, None).await?;
    tracing::info!("Database ready: {database_url}");
    Ok(db)
}

/// For `sqlite:path` URLs, create the parent directory and the database file if they don't exist.
fn ensure_sqlite_dir(url: &str) -> Result<(), DbErr> {
    // Strip "sqlite:" and any leading slashes (e.g. "sqlite:./foo", "sqlite:///abs/foo").
    let after_scheme = match url.strip_prefix("sqlite:") {
        Some(p) => p,
        None => return Ok(()),
    };

    // Reconstruct the absolute/relative file path. One leading slash = absolute; two or more
    // slashes (e.g. "sqlite:///path") also indicate absolute, but strip down to one. Zero
    // leading slashes = relative (e.g. "sqlite:./foo" or "sqlite:foo").
    let path_str = match after_scheme.chars().take_while(|&c| c == '/').count() {
        0 => after_scheme.to_owned(),          // relative: "./db.sqlite"
        1 => after_scheme.to_owned(),          // "/absolute/path"
        n => after_scheme[n - 1..].to_owned(), // "///abs" → "/abs"
    };

    // Skip special targets (:memory: or empty)
    if path_str.is_empty() || path_str == ":memory:" {
        return Ok(());
    }

    let path = std::path::Path::new(&path_str);

    // Create parent directories first.
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DbErr::Custom(format!("cannot create database directory: {e}")))?;
        }
    }

    // Touch the file so SQLite can open it regardless of create_if_missing settings.
    if !path.exists() {
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .map_err(|e| DbErr::Custom(format!("cannot create database file: {e}")))?;
    }

    Ok(())
}
