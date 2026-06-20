use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00007_add_streak_to_user_profiles"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserProfiles::Table)
                    .add_column(
                        ColumnDef::new(UserProfiles::Streak)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite cannot drop columns; recreate the table without streak.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE user_profiles_new (
                    user_id             BIGINT NOT NULL,
                    guild_id            BIGINT NOT NULL,
                    xp                  BIGINT NOT NULL DEFAULT 0,
                    total_voice_seconds BIGINT NOT NULL DEFAULT 0,
                    last_daily_at       BIGINT,
                    PRIMARY KEY (user_id, guild_id),
                    FOREIGN KEY (guild_id) REFERENCES guilds(id) ON DELETE CASCADE
                );
                INSERT INTO user_profiles_new
                    SELECT user_id, guild_id, xp, total_voice_seconds, last_daily_at
                    FROM user_profiles;
                DROP TABLE user_profiles;
                ALTER TABLE user_profiles_new RENAME TO user_profiles;",
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum UserProfiles {
    Table,
    Streak,
}
