use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "voice_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub guild_id: i64,
    pub joined_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
