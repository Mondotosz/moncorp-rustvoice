use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, Set};

use crate::entities::primary_channel::{self, Entity as PrimaryChannel};
use crate::error::DbError;

pub async fn insert(id: i64, guild_id: i64, db: &DatabaseConnection) -> Result<(), DbError> {
    let model = primary_channel::ActiveModel {
        id: Set(id),
        guild_id: Set(guild_id),
    };
    PrimaryChannel::insert(model).exec(db).await?;
    Ok(())
}

pub async fn delete(id: i64, db: &DatabaseConnection) -> Result<(), DbError> {
    PrimaryChannel::delete_by_id(id).exec(db).await?;
    Ok(())
}

pub async fn exists(id: i64, db: &DatabaseConnection) -> Result<bool, DbError> {
    Ok(PrimaryChannel::find_by_id(id).one(db).await?.is_some())
}

pub async fn list_by_guild(
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<Vec<primary_channel::Model>, DbError> {
    Ok(PrimaryChannel::find()
        .filter(primary_channel::Column::GuildId.eq(guild_id))
        .all(db)
        .await?)
}

pub async fn count_by_guild(guild_id: i64, db: &DatabaseConnection) -> Result<u64, DbError> {
    Ok(PrimaryChannel::find()
        .filter(primary_channel::Column::GuildId.eq(guild_id))
        .count(db)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_db;

    #[tokio::test]
    async fn insert_exists_delete_roundtrip() {
        let db = test_db().await;
        crate::repositories::guild::upsert(1, &db).await.unwrap();

        assert!(!exists(100, &db).await.unwrap());
        insert(100, 1, &db).await.unwrap();
        assert!(exists(100, &db).await.unwrap());
        delete(100, &db).await.unwrap();
        assert!(!exists(100, &db).await.unwrap());
    }

    #[tokio::test]
    async fn list_by_guild_filters_by_guild() {
        let db = test_db().await;
        crate::repositories::guild::upsert(1, &db).await.unwrap();
        crate::repositories::guild::upsert(2, &db).await.unwrap();

        insert(100, 1, &db).await.unwrap();
        insert(101, 1, &db).await.unwrap();
        insert(200, 2, &db).await.unwrap();

        let list = list_by_guild(1, &db).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|c| c.guild_id == 1));
    }

    #[tokio::test]
    async fn count_by_guild_filters_by_guild() {
        let db = test_db().await;
        crate::repositories::guild::upsert(1, &db).await.unwrap();
        crate::repositories::guild::upsert(2, &db).await.unwrap();

        insert(100, 1, &db).await.unwrap();
        insert(101, 1, &db).await.unwrap();
        insert(200, 2, &db).await.unwrap();

        assert_eq!(count_by_guild(1, &db).await.unwrap(), 2);
        assert_eq!(count_by_guild(2, &db).await.unwrap(), 1);
    }
}
