use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00008_add_fk_to_voice_sessions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite cannot add FK constraints via ALTER TABLE; recreate the table.
        manager
            .get_connection()
            .execute_unprepared(
                "PRAGMA foreign_keys = OFF;
                CREATE TABLE voice_sessions_new (
                    user_id   BIGINT NOT NULL,
                    guild_id  BIGINT NOT NULL,
                    joined_at BIGINT NOT NULL,
                    PRIMARY KEY (user_id, guild_id),
                    FOREIGN KEY (guild_id) REFERENCES guilds(id) ON DELETE CASCADE
                );
                INSERT INTO voice_sessions_new SELECT * FROM voice_sessions;
                DROP TABLE voice_sessions;
                ALTER TABLE voice_sessions_new RENAME TO voice_sessions;
                PRAGMA foreign_keys = ON;",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE voice_sessions_new (
                    user_id   BIGINT NOT NULL,
                    guild_id  BIGINT NOT NULL,
                    joined_at BIGINT NOT NULL,
                    PRIMARY KEY (user_id, guild_id)
                );
                INSERT INTO voice_sessions_new SELECT * FROM voice_sessions;
                DROP TABLE voice_sessions;
                ALTER TABLE voice_sessions_new RENAME TO voice_sessions;",
            )
            .await?;
        Ok(())
    }
}
