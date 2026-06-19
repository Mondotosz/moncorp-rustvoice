use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "primary_channels")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub guild_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::guild::Entity",
        from = "Column::GuildId",
        to = "super::guild::Column::Id",
        on_delete = "Cascade"
    )]
    Guild,
    #[sea_orm(has_many = "super::temporary_channel::Entity")]
    TemporaryChannel,
}

impl Related<super::guild::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Guild.def()
    }
}

impl Related<super::temporary_channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TemporaryChannel.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
