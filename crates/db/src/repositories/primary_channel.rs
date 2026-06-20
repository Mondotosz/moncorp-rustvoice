use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

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
