use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "temporary_channels")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub guild_id: i64,
    pub primary_channel_id: i64,
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
    #[sea_orm(
        belongs_to = "super::primary_channel::Entity",
        from = "Column::PrimaryChannelId",
        to = "super::primary_channel::Column::Id",
        on_delete = "Cascade"
    )]
    PrimaryChannel,
}

impl Related<super::guild::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Guild.def()
    }
}

impl Related<super::primary_channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PrimaryChannel.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
