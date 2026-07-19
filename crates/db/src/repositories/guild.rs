use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, Set};

use crate::entities::guild::{self, Entity as Guild};
use crate::error::DbError;

pub async fn upsert(id: i64, db: &DatabaseConnection) -> Result<(), DbError> {
    let model = guild::ActiveModel { id: Set(id) };
    match Guild::insert(model)
        .on_conflict(
            OnConflict::column(guild::Column::Id)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await
    {
        Ok(_) | Err(DbErr::RecordNotInserted) => Ok(()),
        Err(e) => Err(DbError::from(e)),
    }
}

pub async fn count(db: &DatabaseConnection) -> Result<u64, DbError> {
    Ok(Guild::find().count(db).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_db;

    #[tokio::test]
    async fn upsert_is_idempotent() {
        let db = test_db().await;
        upsert(1, &db).await.unwrap();
        upsert(1, &db).await.unwrap();
        assert_eq!(count(&db).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn count_reflects_distinct_guilds() {
        let db = test_db().await;
        upsert(1, &db).await.unwrap();
        upsert(2, &db).await.unwrap();
        assert_eq!(count(&db).await.unwrap(), 2);
    }
}
