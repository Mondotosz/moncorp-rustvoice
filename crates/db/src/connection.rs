use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::error::DbError;
use crate::migrator::Migrator;

/// Connect without running migrations — useful for checking migration status.
pub async fn connect_raw(database_url: &str) -> Result<DatabaseConnection, DbError> {
    ensure_sqlite_dir(database_url)?;
    let mut opts = ConnectOptions::new(database_url);
    opts.max_connections(5).sqlx_logging(false);
    Ok(Database::connect(opts).await?)
}

pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbError> {
    ensure_sqlite_dir(database_url)?;

    let mut opts = ConnectOptions::new(database_url);
    opts.max_connections(5).sqlx_logging(false);

    let db = Database::connect(opts).await?;
    Migrator::up(&db, None).await?;
    tracing::info!("Database ready: {database_url}");
    Ok(db)
}

/// Connects to a fresh, fully-migrated in-memory SQLite database for use in tests.
/// Pinned to a single connection: SQLite's `:memory:` database is private to the
/// connection that created it, so a multi-connection pool would silently see an empty
/// database on some queries.
pub async fn connect_in_memory_for_tests() -> Result<DatabaseConnection, DbError> {
    let mut opts = ConnectOptions::new("sqlite::memory:");
    opts.max_connections(1).sqlx_logging(false);
    let db = Database::connect(opts).await?;
    Migrator::up(&db, None).await?;
    Ok(db)
}

/// Resolves a `sqlite:...` connection URL to the filesystem path it points at, or
/// `None` if the URL isn't a `sqlite:` URL or targets a special in-memory database.
///
/// One leading slash after the scheme means absolute (`sqlite:/absolute/path`); two or
/// more (`sqlite:///abs/path`) also mean absolute but get collapsed down to one; zero
/// leading slashes mean relative (`sqlite:./foo`, `sqlite:foo`).
fn resolve_sqlite_path(url: &str) -> Option<String> {
    let after_scheme = url.strip_prefix("sqlite:")?;

    let path_str = match after_scheme.chars().take_while(|&c| c == '/').count() {
        0 => after_scheme.to_owned(),          // relative: "./db.sqlite"
        1 => after_scheme.to_owned(),          // "/absolute/path"
        n => after_scheme[n - 1..].to_owned(), // "///abs" → "/abs"
    };

    if path_str.is_empty() || path_str == ":memory:" {
        return None;
    }

    Some(path_str)
}

/// For `sqlite:path` URLs, create the parent directory and the database file if they don't exist.
fn ensure_sqlite_dir(url: &str) -> Result<(), DbError> {
    let Some(path_str) = resolve_sqlite_path(url) else {
        return Ok(());
    };

    let path = std::path::Path::new(&path_str);

    // Create parent directories first.
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                std::io::Error::new(e.kind(), format!("cannot create {}: {e}", parent.display()))
            })?;
        }
    }

    // Touch the file so SQLite can open it regardless of create_if_missing settings.
    if !path.exists() {
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .map_err(|e| {
                std::io::Error::new(e.kind(), format!("cannot create {}: {e}", path.display()))
            })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_pass_through_unchanged() {
        assert_eq!(
            resolve_sqlite_path("sqlite:./db.sqlite"),
            Some("./db.sqlite".to_owned())
        );
        assert_eq!(
            resolve_sqlite_path("sqlite:db.sqlite"),
            Some("db.sqlite".to_owned())
        );
    }

    #[test]
    fn single_leading_slash_is_absolute() {
        assert_eq!(
            resolve_sqlite_path("sqlite:/absolute/db.sqlite"),
            Some("/absolute/db.sqlite".to_owned())
        );
    }

    #[test]
    fn triple_slash_is_collapsed_to_one() {
        assert_eq!(
            resolve_sqlite_path("sqlite:///absolute/db.sqlite"),
            Some("/absolute/db.sqlite".to_owned())
        );
    }

    #[test]
    fn memory_and_empty_targets_resolve_to_none() {
        assert_eq!(resolve_sqlite_path("sqlite::memory:"), None);
        assert_eq!(resolve_sqlite_path("sqlite:"), None);
    }

    #[test]
    fn non_sqlite_urls_resolve_to_none() {
        assert_eq!(resolve_sqlite_path("postgres://localhost/db"), None);
    }
}
