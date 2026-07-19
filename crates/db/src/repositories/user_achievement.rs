use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set};

use crate::entities::user_achievement::{self, Entity as UserAchievement};
use crate::error::DbError;

/// Records that `achievement_id` was unlocked for this user/guild, unless it already
/// was. Returns `true` if this call newly recorded it, `false` if it was already unlocked.
pub async fn unlock(
    user_id: i64,
    guild_id: i64,
    achievement_id: &str,
    unlocked_at: i64,
    db: &DatabaseConnection,
) -> Result<bool, DbError> {
    let model = user_achievement::ActiveModel {
        user_id: Set(user_id),
        guild_id: Set(guild_id),
        achievement_id: Set(achievement_id.to_owned()),
        unlocked_at: Set(unlocked_at),
    };
    match UserAchievement::insert(model)
        .on_conflict(
            OnConflict::columns([
                user_achievement::Column::UserId,
                user_achievement::Column::GuildId,
                user_achievement::Column::AchievementId,
            ])
            .do_nothing()
            .to_owned(),
        )
        .exec(db)
        .await
    {
        Ok(_) => Ok(true),
        Err(DbErr::RecordNotInserted) => Ok(false),
        Err(e) => Err(DbError::from(e)),
    }
}

pub async fn list_by_user(
    user_id: i64,
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<Vec<user_achievement::Model>, DbError> {
    Ok(UserAchievement::find()
        .filter(user_achievement::Column::UserId.eq(user_id))
        .filter(user_achievement::Column::GuildId.eq(guild_id))
        .all(db)
        .await?)
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
    async fn unlock_reports_newly_unlocked_then_already_unlocked() {
        let db = test_db().await;
        seed_guild(&db, 1).await;

        assert!(unlock(42, 1, "level-10", 1_000, &db).await.unwrap());
        assert!(!unlock(42, 1, "level-10", 2_000, &db).await.unwrap());
    }

    #[tokio::test]
    async fn list_by_user_returns_only_that_users_guild_achievements() {
        let db = test_db().await;
        seed_guild(&db, 1).await;
        seed_guild(&db, 2).await;

        unlock(42, 1, "level-10", 1_000, &db).await.unwrap();
        unlock(42, 1, "streak-7", 1_000, &db).await.unwrap();
        unlock(42, 2, "level-10", 1_000, &db).await.unwrap();
        unlock(99, 1, "level-10", 1_000, &db).await.unwrap();

        let list = list_by_user(42, 1, &db).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|a| a.user_id == 42 && a.guild_id == 1));
    }
}
