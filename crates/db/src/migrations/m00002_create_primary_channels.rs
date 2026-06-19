use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00002_create_primary_channels"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PrimaryChannels::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PrimaryChannels::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PrimaryChannels::GuildId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(PrimaryChannels::Table, PrimaryChannels::GuildId)
                            .to(Guilds::Table, Guilds::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PrimaryChannels::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Guilds {
    Table,
    Id,
}

#[derive(DeriveIden)]
pub enum PrimaryChannels {
    Table,
    Id,
    GuildId,
}
