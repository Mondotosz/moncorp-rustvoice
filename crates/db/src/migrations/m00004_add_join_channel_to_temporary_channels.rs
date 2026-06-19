use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00004_add_join_channel_to_temporary_channels"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TemporaryChannels::Table)
                    .add_column(
                        ColumnDef::new(TemporaryChannels::JoinChannelId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite cannot drop columns; recreate the table without join_channel_id.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE temporary_channels_new (
                    id              BIGINT NOT NULL PRIMARY KEY,
                    guild_id        BIGINT NOT NULL,
                    primary_channel_id BIGINT NOT NULL
                );
                INSERT INTO temporary_channels_new (id, guild_id, primary_channel_id)
                    SELECT id, guild_id, primary_channel_id FROM temporary_channels;
                DROP TABLE temporary_channels;
                ALTER TABLE temporary_channels_new RENAME TO temporary_channels;",
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum TemporaryChannels {
    Table,
    JoinChannelId,
}
