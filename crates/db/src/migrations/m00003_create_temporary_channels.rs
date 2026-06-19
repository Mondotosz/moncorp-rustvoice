use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00003_create_temporary_channels"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TemporaryChannels::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TemporaryChannels::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TemporaryChannels::GuildId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TemporaryChannels::PrimaryChannelId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TemporaryChannels::Table, TemporaryChannels::GuildId)
                            .to(Guilds::Table, Guilds::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                TemporaryChannels::Table,
                                TemporaryChannels::PrimaryChannelId,
                            )
                            .to(PrimaryChannels::Table, PrimaryChannels::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TemporaryChannels::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Guilds {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PrimaryChannels {
    Table,
    Id,
}

#[derive(DeriveIden)]
pub enum TemporaryChannels {
    Table,
    Id,
    GuildId,
    PrimaryChannelId,
}
