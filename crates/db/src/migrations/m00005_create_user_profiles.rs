use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00005_create_user_profiles"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserProfiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserProfiles::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserProfiles::GuildId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserProfiles::Xp)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(UserProfiles::TotalVoiceSeconds)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(UserProfiles::LastDailyAt)
                            .big_integer()
                            .null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(UserProfiles::UserId)
                            .col(UserProfiles::GuildId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserProfiles::Table, UserProfiles::GuildId)
                            .to(Guilds::Table, Guilds::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserProfiles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Guilds {
    Table,
    Id,
}

#[derive(DeriveIden)]
pub enum UserProfiles {
    Table,
    UserId,
    GuildId,
    Xp,
    TotalVoiceSeconds,
    LastDailyAt,
}
