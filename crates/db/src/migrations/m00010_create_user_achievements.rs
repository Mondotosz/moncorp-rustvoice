use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00010_create_user_achievements"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserAchievements::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserAchievements::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserAchievements::GuildId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserAchievements::AchievementId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserAchievements::UnlockedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(UserAchievements::UserId)
                            .col(UserAchievements::GuildId)
                            .col(UserAchievements::AchievementId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserAchievements::Table, UserAchievements::GuildId)
                            .to(Guilds::Table, Guilds::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserAchievements::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Guilds {
    Table,
    Id,
}

#[derive(DeriveIden)]
pub enum UserAchievements {
    Table,
    UserId,
    GuildId,
    AchievementId,
    UnlockedAt,
}
