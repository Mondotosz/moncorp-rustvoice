use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, QueryFilter, Set,
};

use crate::entities::temporary_channel::{self, Entity as TemporaryChannel};

pub async fn insert(
    id: i64,
    guild_id: i64,
    primary_channel_id: i64,
    db: &DatabaseConnection,
) -> Result<(), DbErr> {
    let model = temporary_channel::ActiveModel {
        id: Set(id),
        guild_id: Set(guild_id),
        primary_channel_id: Set(primary_channel_id),
    };
    TemporaryChannel::insert(model).exec(db).await?;
    Ok(())
}

pub async fn delete(id: i64, db: &DatabaseConnection) -> Result<(), DbErr> {
    TemporaryChannel::delete_by_id(id).exec(db).await?;
    Ok(())
}

pub async fn exists(id: i64, db: &DatabaseConnection) -> Result<bool, DbErr> {
    Ok(TemporaryChannel::find_by_id(id).one(db).await?.is_some())
}

pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<temporary_channel::Model>, DbErr> {
    TemporaryChannel::find().all(db).await
}

pub async fn count_all(db: &DatabaseConnection) -> Result<u64, DbErr> {
    TemporaryChannel::find().count(db).await
}

pub async fn list_by_guild(
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<Vec<temporary_channel::Model>, DbErr> {
    TemporaryChannel::find()
        .filter(temporary_channel::Column::GuildId.eq(guild_id))
        .all(db)
        .await
}

pub async fn count_by_guild(guild_id: i64, db: &DatabaseConnection) -> Result<u64, DbErr> {
    TemporaryChannel::find()
        .filter(temporary_channel::Column::GuildId.eq(guild_id))
        .count(db)
        .await
}
