use sqlx::{sqlite::SqliteQueryResult, Pool, Sqlite};

#[derive(Clone, sqlx::FromRow, Debug)]
pub struct PrimaryChannel {
    pub id: i64,
}

impl PrimaryChannel {
    pub async fn insert(id: i64, db: &Pool<Sqlite>) -> Result<SqliteQueryResult, sqlx::Error> {
        sqlx::query!("INSERT INTO primary_channels (id) VALUES (?)", id)
            .execute(db)
            .await
    }

    pub async fn exists(id: i64, db: &Pool<Sqlite>) -> Result<bool, sqlx::Error> {
        Ok(
            sqlx::query!("SELECT id FROM primary_channels WHERE id = ?", id)
                .fetch_optional(db)
                .await?
                .is_some(),
        )
    }
}

#[derive(Clone, sqlx::FromRow, Debug)]
pub struct TemporaryChannel {
    pub id: i64,
}

impl TemporaryChannel {
    pub async fn insert(id: i64, db: &Pool<Sqlite>) -> Result<SqliteQueryResult, sqlx::Error> {
        sqlx::query!("INSERT INTO temporary_channels (id) VALUES (?)", id)
            .execute(db)
            .await
    }

    pub async fn delete(id: i64, db: &Pool<Sqlite>) -> Result<SqliteQueryResult, sqlx::Error> {
        sqlx::query!("DELETE FROM temporary_channels WHERE id = ?", id)
            .execute(db)
            .await
    }

    pub async fn exists(id: i64, db: &Pool<Sqlite>) -> Result<bool, sqlx::Error> {
        Ok(
            sqlx::query!("SELECT id FROM temporary_channels WHERE id = ?", id)
                .fetch_optional(db)
                .await?
                .is_some(),
        )
    }
}
