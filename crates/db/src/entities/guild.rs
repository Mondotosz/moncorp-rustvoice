use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "guilds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::primary_channel::Entity")]
    PrimaryChannel,
    #[sea_orm(has_many = "super::temporary_channel::Entity")]
    TemporaryChannel,
}

impl Related<super::primary_channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PrimaryChannel.def()
    }
}

impl Related<super::temporary_channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TemporaryChannel.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
