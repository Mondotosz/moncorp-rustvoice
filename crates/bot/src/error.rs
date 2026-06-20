use poise::serenity_prelude as serenity;

use crate::permissions::BotPermissionError;

#[derive(Debug, thiserror::Error)]
pub enum BotError {
    #[error(transparent)]
    Serenity(#[from] serenity::Error),
    #[error(transparent)]
    Db(#[from] db::DbError),
    #[error(transparent)]
    Permission(#[from] BotPermissionError),
    #[error("{0}")]
    Other(String),
}
