use std::collections::HashSet;

use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set};

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
            OnConflict::columns([
                voice_session::Column::UserId,
                voice_session::Column::GuildId,
            ])
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
    let active: HashSet<i64> = active_user_ids.iter().copied().collect();
    let sessions = list_by_guild(guild_id, db).await?;
    for session in sessions {
        if !active.contains(&session.user_id) {
            VoiceSession::delete_many()
                .filter(voice_session::Column::UserId.eq(session.user_id))
                .filter(voice_session::Column::GuildId.eq(guild_id))
                .exec(db)
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_db;

    async fn seed_guild(db: &DatabaseConnection, guild_id: i64) {
        crate::repositories::guild::upsert(guild_id, db)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn start_end_roundtrip_returns_joined_at() {
        let db = test_db().await;
        seed_guild(&db, 1).await;

        start(42, 1, 1_000, &db).await.unwrap();
        let joined_at = end(42, 1, &db).await.unwrap();
        assert_eq!(joined_at, Some(1_000));

        // Session was removed by `end`.
        assert_eq!(end(42, 1, &db).await.unwrap(), None);
    }

    #[tokio::test]
    async fn start_does_not_overwrite_an_existing_session() {
        let db = test_db().await;
        seed_guild(&db, 1).await;

        start(42, 1, 1_000, &db).await.unwrap();
        start(42, 1, 2_000, &db).await.unwrap(); // reconnect race — should be a no-op

        assert_eq!(end(42, 1, &db).await.unwrap(), Some(1_000));
    }

    #[tokio::test]
    async fn delete_orphaned_only_removes_inactive_sessions() {
        let db = test_db().await;
        seed_guild(&db, 1).await;

        start(1, 1, 1_000, &db).await.unwrap();
        start(2, 1, 1_000, &db).await.unwrap();
        start(3, 1, 1_000, &db).await.unwrap();

        delete_orphaned(1, &[1, 3], &db).await.unwrap();

        let mut remaining: Vec<i64> = list_by_guild(1, &db)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.user_id)
            .collect();
        remaining.sort();
        assert_eq!(remaining, vec![1, 3]);
    }
}
