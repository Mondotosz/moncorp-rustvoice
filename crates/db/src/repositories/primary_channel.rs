use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set};

use crate::entities::primary_channel::{self, Entity as PrimaryChannel};

pub async fn insert(id: i64, guild_id: i64, db: &DatabaseConnection) -> Result<(), DbErr> {
    let model = primary_channel::ActiveModel {
        id: Set(id),
        guild_id: Set(guild_id),
    };
    PrimaryChannel::insert(model).exec(db).await?;
    Ok(())
}

pub async fn delete(id: i64, db: &DatabaseConnection) -> Result<(), DbErr> {
    PrimaryChannel::delete_by_id(id).exec(db).await?;
    Ok(())
}

pub async fn exists(id: i64, db: &DatabaseConnection) -> Result<bool, DbErr> {
    Ok(PrimaryChannel::find_by_id(id).one(db).await?.is_some())
}

pub async fn list_by_guild(
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<Vec<primary_channel::Model>, DbErr> {
    PrimaryChannel::find()
        .filter(primary_channel::Column::GuildId.eq(guild_id))
        .all(db)
        .await
}
