use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, Set};

use crate::entities::guild::{self, Entity as Guild};
use crate::error::DbError;

pub async fn upsert(id: i64, db: &DatabaseConnection) -> Result<(), DbError> {
    let model = guild::ActiveModel {
        id: Set(id),
        channel_name_template: Set(None),
    };
    match Guild::insert(model)
        .on_conflict(
            OnConflict::column(guild::Column::Id)
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

pub async fn count(db: &DatabaseConnection) -> Result<u64, DbError> {
    Ok(Guild::find().count(db).await?)
}

pub async fn channel_name_template(
    id: i64,
    db: &DatabaseConnection,
) -> Result<Option<String>, DbError> {
    Ok(Guild::find_by_id(id)
        .one(db)
        .await?
        .and_then(|m| m.channel_name_template))
}

pub async fn set_channel_name_template(
    id: i64,
    template: Option<String>,
    db: &DatabaseConnection,
) -> Result<(), DbError> {
    let model = guild::ActiveModel {
        id: Set(id),
        channel_name_template: Set(template),
    };
    Guild::insert(model)
        .on_conflict(
            OnConflict::column(guild::Column::Id)
                .update_column(guild::Column::ChannelNameTemplate)
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_db;

    #[tokio::test]
    async fn upsert_is_idempotent() {
        let db = test_db().await;
        upsert(1, &db).await.unwrap();
        upsert(1, &db).await.unwrap();
        assert_eq!(count(&db).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn count_reflects_distinct_guilds() {
        let db = test_db().await;
        upsert(1, &db).await.unwrap();
        upsert(2, &db).await.unwrap();
        assert_eq!(count(&db).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn channel_name_template_defaults_to_none() {
        let db = test_db().await;
        upsert(1, &db).await.unwrap();
        assert_eq!(channel_name_template(1, &db).await.unwrap(), None);
    }

    #[tokio::test]
    async fn channel_name_template_returns_none_for_unregistered_guild() {
        let db = test_db().await;
        assert_eq!(channel_name_template(1, &db).await.unwrap(), None);
    }

    #[tokio::test]
    async fn set_channel_name_template_creates_the_guild_row_if_missing() {
        let db = test_db().await;
        set_channel_name_template(1, Some("[{game}]".to_string()), &db)
            .await
            .unwrap();
        assert_eq!(
            channel_name_template(1, &db).await.unwrap(),
            Some("[{game}]".to_string())
        );
        assert_eq!(count(&db).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn set_channel_name_template_overwrites_existing_value() {
        let db = test_db().await;
        upsert(1, &db).await.unwrap();
        set_channel_name_template(1, Some("🎮 {game}".to_string()), &db)
            .await
            .unwrap();
        set_channel_name_template(1, Some("[{game}]".to_string()), &db)
            .await
            .unwrap();
        assert_eq!(
            channel_name_template(1, &db).await.unwrap(),
            Some("[{game}]".to_string())
        );
    }

    #[tokio::test]
    async fn set_channel_name_template_can_clear_back_to_none() {
        let db = test_db().await;
        set_channel_name_template(1, Some("[{game}]".to_string()), &db)
            .await
            .unwrap();
        set_channel_name_template(1, None, &db).await.unwrap();
        assert_eq!(channel_name_template(1, &db).await.unwrap(), None);
    }
}
