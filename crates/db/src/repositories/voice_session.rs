use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set};
use sea_orm::sea_query::OnConflict;

use crate::entities::voice_session::{self, Entity as VoiceSession};
use crate::error::DbError;

pub async fn start(
    user_id: i64,
    guild_id: i64,
    joined_at: i64,
    db: &DatabaseConnection,
) -> Result<(), DbError> {
    let model = voice_session::ActiveModel {
        user_id: Set(user_id),
        guild_id: Set(guild_id),
        joined_at: Set(joined_at),
    };
    match VoiceSession::insert(model)
        .on_conflict(
            OnConflict::columns([voice_session::Column::UserId, voice_session::Column::GuildId])
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

pub async fn end(
    user_id: i64,
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<Option<i64>, DbError> {
    let session = VoiceSession::find()
        .filter(voice_session::Column::UserId.eq(user_id))
        .filter(voice_session::Column::GuildId.eq(guild_id))
        .one(db)
        .await?;

    let Some(session) = session else {
        return Ok(None);
    };
    let joined_at = session.joined_at;

    VoiceSession::delete_many()
        .filter(voice_session::Column::UserId.eq(user_id))
        .filter(voice_session::Column::GuildId.eq(guild_id))
        .exec(db)
        .await?;

    Ok(Some(joined_at))
}

pub async fn list_by_guild(
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<Vec<voice_session::Model>, DbError> {
    Ok(VoiceSession::find()
        .filter(voice_session::Column::GuildId.eq(guild_id))
        .all(db)
        .await?)
}

pub async fn delete_orphaned(
    guild_id: i64,
    active_user_ids: &[i64],
    db: &DatabaseConnection,
) -> Result<(), DbError> {
    let sessions = list_by_guild(guild_id, db).await?;
    for session in sessions {
        if !active_user_ids.contains(&session.user_id) {
            VoiceSession::delete_many()
                .filter(voice_session::Column::UserId.eq(session.user_id))
                .filter(voice_session::Column::GuildId.eq(guild_id))
                .exec(db)
                .await?;
        }
    }
    Ok(())
}
