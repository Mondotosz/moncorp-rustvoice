use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
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

/// Sum of `total_voice_seconds` across every member with a profile in `guild_id`.
pub async fn total_voice_seconds_by_guild(
    guild_id: i64,
    db: &DatabaseConnection,
) -> Result<i64, DbError> {
    let total: Option<Option<i64>> = UserProfile::find()
        .filter(user_profile::Column::GuildId.eq(guild_id))
        .select_only()
        .column_as(user_profile::Column::TotalVoiceSeconds.sum(), "total")
        .into_tuple()
        .one(db)
        .await?;
    Ok(total.flatten().unwrap_or(0))
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
    async fn get_returns_none_before_any_activity() {
        let db = test_db().await;
        seed_guild(&db, 1).await;
        assert!(get(42, 1, &db).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn add_xp_creates_and_accumulates() {
        let db = test_db().await;
        seed_guild(&db, 1).await;

        add_xp(42, 1, 100, 100, &db).await.unwrap();
        add_xp(42, 1, 50, 50, &db).await.unwrap();

        let profile = get(42, 1, &db).await.unwrap().unwrap();
        assert_eq!(profile.xp, 150);
        assert_eq!(profile.total_voice_seconds, 150);
    }

    #[tokio::test]
    async fn total_voice_seconds_by_guild_is_zero_with_no_profiles() {
        let db = test_db().await;
        seed_guild(&db, 1).await;
        assert_eq!(total_voice_seconds_by_guild(1, &db).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn total_voice_seconds_by_guild_sums_only_that_guild() {
        let db = test_db().await;
        seed_guild(&db, 1).await;
        seed_guild(&db, 2).await;

        add_xp(1, 1, 0, 100, &db).await.unwrap();
        add_xp(2, 1, 0, 250, &db).await.unwrap();
        add_xp(3, 2, 0, 999, &db).await.unwrap();

        assert_eq!(total_voice_seconds_by_guild(1, &db).await.unwrap(), 350);
        assert_eq!(total_voice_seconds_by_guild(2, &db).await.unwrap(), 999);
    }

    #[tokio::test]
    async fn list_top_by_guild_orders_by_xp_desc() {
        let db = test_db().await;
        seed_guild(&db, 1).await;

        add_xp(1, 1, 50, 0, &db).await.unwrap();
        add_xp(2, 1, 200, 0, &db).await.unwrap();
        add_xp(3, 1, 100, 0, &db).await.unwrap();

        let ranked = list_top_by_guild(1, &db).await.unwrap();
        let ids: Vec<i64> = ranked.iter().map(|p| p.user_id).collect();
        assert_eq!(ids, vec![2, 3, 1]);
    }

    #[tokio::test]
    async fn set_daily_state_persists_streak_and_timestamp() {
        let db = test_db().await;
        seed_guild(&db, 1).await;

        set_daily_state(42, 1, 1_000, 3, &db).await.unwrap();

        let profile = get(42, 1, &db).await.unwrap().unwrap();
        assert_eq!(profile.last_daily_at, Some(1_000));
        assert_eq!(profile.streak, 3);
    }
}
