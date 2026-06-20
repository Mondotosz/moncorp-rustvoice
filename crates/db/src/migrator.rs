use sea_orm_migration::prelude::*;

use crate::migrations::{
    m00001_create_guilds, m00002_create_primary_channels, m00003_create_temporary_channels,
    m00004_add_join_channel_to_temporary_channels, m00005_create_user_profiles,
    m00006_create_voice_sessions, m00007_add_streak_to_user_profiles,
};

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_guilds::Migration),
            Box::new(m00002_create_primary_channels::Migration),
            Box::new(m00003_create_temporary_channels::Migration),
            Box::new(m00004_add_join_channel_to_temporary_channels::Migration),
            Box::new(m00005_create_user_profiles::Migration),
            Box::new(m00006_create_voice_sessions::Migration),
            Box::new(m00007_add_streak_to_user_profiles::Migration),
        ]
    }
}
