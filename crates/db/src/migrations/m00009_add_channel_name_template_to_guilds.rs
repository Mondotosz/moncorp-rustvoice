use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00009_add_channel_name_template_to_guilds"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Guilds::Table)
                    .add_column(ColumnDef::new(Guilds::ChannelNameTemplate).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite cannot drop columns; recreate the table without channel_name_template.
        // guilds is a parent table for several FKs, so foreign_keys must be off for the swap.
        manager
            .get_connection()
            .execute_unprepared(
                "PRAGMA foreign_keys = OFF;
                CREATE TABLE guilds_new (
                    id BIGINT NOT NULL PRIMARY KEY
                );
                INSERT INTO guilds_new (id) SELECT id FROM guilds;
                DROP TABLE guilds;
                ALTER TABLE guilds_new RENAME TO guilds;
                PRAGMA foreign_keys = ON;",
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Guilds {
    Table,
    ChannelNameTemplate,
}
