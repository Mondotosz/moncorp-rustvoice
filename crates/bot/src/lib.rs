use std::sync::{Arc, OnceLock};

use poise::serenity_prelude as serenity;

use db::DatabaseConnection;

pub mod activity;
pub mod client;
pub mod commands;
pub mod error;
pub mod events;
pub mod ipc_server;
pub mod leveling;
pub mod permissions;

/// HTTP + cache handles shared between the IPC server and event handlers.
/// Populated once the bot fires its Ready event.
pub struct BotContext {
    pub http: Arc<serenity::Http>,
    pub cache: Arc<serenity::Cache>,
}

/// Shared state available to every poise command through the bot's [`Context`].
pub struct Data {
    pub db: DatabaseConnection,
    pub start_time: std::time::Instant,
    pub owner_id: Option<serenity::UserId>,
}

pub use error::BotError;

/// Concrete error type returned by all command functions and public bot APIs.
pub type Error = BotError;
/// Poise context for guild-only slash commands, parameterised with [`Data`] and [`Error`].
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Generate the OAuth2 invite URL for this bot using the given token.
pub async fn invite_url(token: &str) -> Result<String, Error> {
    let http = poise::serenity_prelude::Http::new(token);
    let user = http.get_current_user().await?;
    let perms = permissions::ALL.bits();
    Ok(format!(
        "https://discord.com/oauth2/authorize?client_id={}&permissions={}&scope=bot+applications.commands",
        user.id, perms
    ))
}

/// Start the Discord bot and IPC server, blocking until shutdown.
pub async fn run(token: String, db: DatabaseConnection, socket_path: String) -> Result<(), Error> {
    let start_time = std::time::Instant::now();

    let owner_id = std::env::var("DISCORD_OWNER_ID")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(serenity::UserId::new);
    if owner_id.is_none() {
        tracing::warn!("DISCORD_OWNER_ID not set — /register will be inaccessible");
    }

    let bot_ctx: Arc<OnceLock<BotContext>> = Arc::new(OnceLock::new());

    let ipc_db = db.clone();
    let ipc_bot_ctx = bot_ctx.clone();
    tokio::spawn(ipc_server::serve(
        socket_path,
        ipc_db,
        start_time,
        ipc_bot_ctx,
    ));

    client::build_and_run(
        token,
        Data {
            db,
            start_time,
            owner_id,
        },
        bot_ctx,
    )
    .await
}

/// Register slash commands without starting the full bot.
/// Uses guild registration (instant) when `guild_id` is supplied, global otherwise.
pub async fn register_commands(token: &str, guild_id: Option<u64>) -> Result<(), Error> {
    let http = serenity::Http::new(token);
    let current_user = http.get_current_user().await?;
    http.set_application_id(serenity::ApplicationId::new(current_user.id.get()));
    let commands = client::all_commands();
    let create_cmds = poise::builtins::create_application_commands(&commands);

    if let Some(id) = guild_id {
        serenity::GuildId::new(id)
            .set_commands(&http, create_cmds)
            .await?;
        tracing::info!("Commands registered in guild {id}");
    } else {
        serenity::Command::set_global_commands(&http, create_cmds).await?;
        tracing::info!("Commands registered globally");
    }
    Ok(())
}
