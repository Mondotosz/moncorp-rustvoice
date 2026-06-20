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
