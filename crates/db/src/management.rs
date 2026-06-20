use sea_orm::DatabaseConnection;
use sea_orm_migration::MigratorTrait;

use crate::error::DbError;
use crate::migrator::Migrator;

pub async fn status(db: &DatabaseConnection) -> Result<(), DbError> {
    let migrations = Migrator::get_migration_with_status(db).await?;
    let total = migrations.len();
    let applied = migrations
        .iter()
        .filter(|m| m.status() == sea_orm_migration::MigrationStatus::Applied)
        .count();

    println!("{:<44} Status", "Migration");
    println!("{}", "-".repeat(54));
    for m in &migrations {
        println!("{:<44} {}", m.name(), m.status());
    }
    println!("{}", "-".repeat(54));
    println!("{applied}/{total} applied");
    Ok(())
}

pub async fn up(db: &DatabaseConnection, steps: Option<u32>) -> Result<(), DbError> {
    Ok(Migrator::up(db, steps).await?)
}

pub async fn down(db: &DatabaseConnection, steps: Option<u32>) -> Result<(), DbError> {
    Ok(Migrator::down(db, steps).await?)
}

pub async fn fresh(db: &DatabaseConnection) -> Result<(), DbError> {
    Ok(Migrator::fresh(db).await?)
}

pub async fn refresh(db: &DatabaseConnection) -> Result<(), DbError> {
    Ok(Migrator::refresh(db).await?)
}

pub async fn reset(db: &DatabaseConnection) -> Result<(), DbError> {
    Ok(Migrator::reset(db).await?)
}

/// Returns `None` if the database is up to date, or `Some(n)` with the number of pending
/// migrations. If the migrations table does not exist yet, all migrations are counted as pending.
pub async fn needs_migration(database_url: &str) -> Result<Option<usize>, DbError> {
    let db = crate::connection::connect_raw(database_url).await?;
    match Migrator::get_pending_migrations(&db).await {
        Ok(pending) if pending.is_empty() => Ok(None),
        Ok(pending) => Ok(Some(pending.len())),
        Err(_) => Ok(Some(Migrator::migrations().len())),
    }
}
