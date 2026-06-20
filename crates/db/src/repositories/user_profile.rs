use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::entities::user_profile::{self, Entity as UserProfile};
use crate::error::DbError;

pub async fn upsert(user_id: i64, guild_id: i64, db: &DatabaseConnection) -> Result<(), DbError> {
    let model = user_profile::ActiveModel {
        user_id: Set(user_id),
        guild_id: Set(guild_id),
        xp: Set(0),
        total_voice_seconds: Set(0),
        last_daily_at: Set(None),
        streak: Set(0),
    };
    match UserProfile::insert(model)
        .on_conflict(
            OnConflict::columns([user_profile::Column::UserId, user_profile::Column::GuildId])
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

pub async fn get(
    user_id: i64,
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<Option<user_profile::Model>, DbError> {
    Ok(UserProfile::find()
        .filter(user_profile::Column::UserId.eq(user_id))
        .filter(user_profile::Column::GuildId.eq(guild_id))
        .one(db)
        .await?)
}

pub async fn add_xp(
    user_id: i64,
    guild_id: i64,
    xp_delta: i64,
    seconds_delta: i64,
    db: &DatabaseConnection,
) -> Result<(), DbError> {
    upsert(user_id, guild_id, db).await?;
    UserProfile::update_many()
        .col_expr(
            user_profile::Column::Xp,
            Expr::col(user_profile::Column::Xp).add(xp_delta),
        )
        .col_expr(
            user_profile::Column::TotalVoiceSeconds,
            Expr::col(user_profile::Column::TotalVoiceSeconds).add(seconds_delta),
        )
        .filter(user_profile::Column::UserId.eq(user_id))
        .filter(user_profile::Column::GuildId.eq(guild_id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn list_top_by_guild(
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<Vec<user_profile::Model>, DbError> {
    Ok(UserProfile::find()
        .filter(user_profile::Column::GuildId.eq(guild_id))
        .order_by_desc(user_profile::Column::Xp)
        .all(db)
        .await?)
}

pub async fn set_daily_state(
    user_id: i64,
    guild_id: i64,
    last_daily_at: i64,
    streak: i64,
    db: &DatabaseConnection,
) -> Result<(), DbError> {
    upsert(user_id, guild_id, db).await?;
    let model = user_profile::ActiveModel {
        user_id: Set(user_id),
        guild_id: Set(guild_id),
        last_daily_at: Set(Some(last_daily_at)),
        streak: Set(streak),
        ..Default::default()
    };
    model.update(db).await?;
    Ok(())
}
