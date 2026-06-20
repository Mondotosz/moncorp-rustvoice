use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00006_create_voice_sessions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(VoiceSessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(VoiceSessions::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VoiceSessions::GuildId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VoiceSessions::JoinedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(VoiceSessions::UserId)
                            .col(VoiceSessions::GuildId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(VoiceSessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum VoiceSessions {
    Table,
    UserId,
    GuildId,
    JoinedAt,
}
