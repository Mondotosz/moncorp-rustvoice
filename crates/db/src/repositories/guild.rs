use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, Set};

use crate::entities::guild::{self, Entity as Guild};
use crate::error::DbError;

pub async fn upsert(id: i64, db: &DatabaseConnection) -> Result<(), DbError> {
    let model = guild::ActiveModel { id: Set(id) };
    Guild::insert(model)
        .on_conflict(
            OnConflict::column(guild::Column::Id)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

pub async fn count(db: &DatabaseConnection) -> Result<u64, DbError> {
    Ok(Guild::find().count(db).await?)
}
