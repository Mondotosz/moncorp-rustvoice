use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};

use crate::entities::temporary_channel::{self, Entity as TemporaryChannel};
use crate::error::DbError;

pub async fn insert(
    id: i64,
    guild_id: i64,
    primary_channel_id: i64,
    db: &DatabaseConnection,
) -> Result<(), DbError> {
    let model = temporary_channel::ActiveModel {
        id: Set(id),
        guild_id: Set(guild_id),
        primary_channel_id: Set(primary_channel_id),
        join_channel_id: Set(None),
    };
    TemporaryChannel::insert(model).exec(db).await?;
    Ok(())
}

pub async fn find(
    id: i64,
    db: &DatabaseConnection,
) -> Result<Option<temporary_channel::Model>, DbError> {
    Ok(TemporaryChannel::find_by_id(id).one(db).await?)
}

pub async fn find_by_join_channel(
    join_channel_id: i64,
    db: &DatabaseConnection,
) -> Result<Option<temporary_channel::Model>, DbError> {
    Ok(TemporaryChannel::find()
        .filter(temporary_channel::Column::JoinChannelId.eq(join_channel_id))
        .one(db)
        .await?)
}

pub async fn set_join_channel(
    id: i64,
    join_channel_id: Option<i64>,
    db: &DatabaseConnection,
) -> Result<(), DbError> {
    let model = temporary_channel::ActiveModel {
        id: Set(id),
        join_channel_id: Set(join_channel_id),
        ..Default::default()
    };
    model.update(db).await?;
    Ok(())
}

pub async fn delete(id: i64, db: &DatabaseConnection) -> Result<(), DbError> {
    TemporaryChannel::delete_by_id(id).exec(db).await?;
    Ok(())
}

pub async fn exists(id: i64, db: &DatabaseConnection) -> Result<bool, DbError> {
    Ok(TemporaryChannel::find_by_id(id).one(db).await?.is_some())
}

pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<temporary_channel::Model>, DbError> {
    Ok(TemporaryChannel::find().all(db).await?)
}

pub async fn count_all(db: &DatabaseConnection) -> Result<u64, DbError> {
    Ok(TemporaryChannel::find().count(db).await?)
}

pub async fn list_by_guild(
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<Vec<temporary_channel::Model>, DbError> {
    Ok(TemporaryChannel::find()
        .filter(temporary_channel::Column::GuildId.eq(guild_id))
        .all(db)
        .await?)
}

pub async fn list_by_primary_channel(
    primary_channel_id: i64,
    db: &DatabaseConnection,
) -> Result<Vec<temporary_channel::Model>, DbError> {
    Ok(TemporaryChannel::find()
        .filter(temporary_channel::Column::PrimaryChannelId.eq(primary_channel_id))
        .all(db)
        .await?)
}

pub async fn count_by_guild(guild_id: i64, db: &DatabaseConnection) -> Result<u64, DbError> {
    Ok(TemporaryChannel::find()
        .filter(temporary_channel::Column::GuildId.eq(guild_id))
        .count(db)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_db;

    /// Seeds a guild and a primary (trigger) channel, returning their ids.
    async fn seed(db: &DatabaseConnection, guild_id: i64, primary_id: i64) {
        crate::repositories::guild::upsert(guild_id, db)
            .await
            .unwrap();
        crate::repositories::primary_channel::insert(primary_id, guild_id, db)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn insert_find_delete_roundtrip() {
        let db = test_db().await;
        seed(&db, 1, 10).await;

        assert!(find(100, &db).await.unwrap().is_none());
        insert(100, 1, 10, &db).await.unwrap();

        let found = find(100, &db).await.unwrap().unwrap();
        assert_eq!(found.guild_id, 1);
        assert_eq!(found.primary_channel_id, 10);
        assert_eq!(found.join_channel_id, None);
        assert!(exists(100, &db).await.unwrap());

        delete(100, &db).await.unwrap();
        assert!(find(100, &db).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn set_and_clear_join_channel() {
        let db = test_db().await;
        seed(&db, 1, 10).await;
        insert(100, 1, 10, &db).await.unwrap();

        set_join_channel(100, Some(999), &db).await.unwrap();
        assert_eq!(
            find(100, &db).await.unwrap().unwrap().join_channel_id,
            Some(999)
        );

        set_join_channel(100, None, &db).await.unwrap();
        assert_eq!(find(100, &db).await.unwrap().unwrap().join_channel_id, None);
    }

    #[tokio::test]
    async fn find_by_join_channel_looks_up_the_companion_channel() {
        let db = test_db().await;
        seed(&db, 1, 10).await;
        insert(100, 1, 10, &db).await.unwrap();
        set_join_channel(100, Some(999), &db).await.unwrap();

        let found = find_by_join_channel(999, &db).await.unwrap().unwrap();
        assert_eq!(found.id, 100);

        assert!(find_by_join_channel(12345, &db).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_and_count_by_guild_and_primary_channel() {
        let db = test_db().await;
        seed(&db, 1, 10).await;
        seed(&db, 1, 11).await;
        seed(&db, 2, 20).await;

        insert(100, 1, 10, &db).await.unwrap();
        insert(101, 1, 11, &db).await.unwrap();
        insert(200, 2, 20, &db).await.unwrap();

        assert_eq!(count_all(&db).await.unwrap(), 3);
        assert_eq!(list_all(&db).await.unwrap().len(), 3);

        assert_eq!(count_by_guild(1, &db).await.unwrap(), 2);
        assert_eq!(list_by_guild(1, &db).await.unwrap().len(), 2);

        let from_primary_10 = list_by_primary_channel(10, &db).await.unwrap();
        assert_eq!(from_primary_10.len(), 1);
        assert_eq!(from_primary_10[0].id, 100);
    }
}
