use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

use poise::serenity_prelude as serenity;

use db::DatabaseConnection;

pub mod activity;
pub mod client;
pub mod commands;
mod context_ext;
pub mod error;
pub mod events;
pub mod ipc_server;
pub mod leveling;
pub mod permissions;
pub mod time;

/// HTTP + cache handles shared between the IPC server and event handlers.
/// Populated once the bot fires its Ready event.
pub struct BotContext {
    pub http: Arc<serenity::Http>,
    pub cache: Arc<serenity::Cache>,
    pub shard_manager: Arc<serenity::ShardManager>,
}

/// Channels currently undergoing an exclusive operation (e.g. `/private`, `/public`),
/// used to prevent concurrent invocations from racing on channel creation/deletion.
pub type ChannelLocks = Mutex<HashSet<serenity::ChannelId>>;

/// Shared state available to every poise command through the bot's [`Context`].
pub struct Data {
    pub db: DatabaseConnection,
    pub start_time: std::time::Instant,
    pub owner_id: Option<serenity::UserId>,
    pub channel_locks: ChannelLocks,
    /// App-level fallback channel-name template, from `DEFAULT_CHANNEL_NAME_TEMPLATE`
    /// or [`activity::DEFAULT_CHANNEL_NAME_TEMPLATE`]. Guilds may override via `/config`.
    pub default_channel_name_template: String,
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

    let owner_id = match std::env::var("DISCORD_OWNER_ID") {
        Err(_) => {
            tracing::warn!("DISCORD_OWNER_ID not set — /register will be inaccessible");
            None
        }
        Ok(val) => match val.parse::<u64>() {
            Ok(id) => Some(serenity::UserId::new(id)),
            Err(_) => {
                tracing::warn!(
                    "DISCORD_OWNER_ID='{}' is not a valid user ID — /register will be inaccessible",
                    val
                );
                None
            }
        },
    };

    let default_channel_name_template = std::env::var("DEFAULT_CHANNEL_NAME_TEMPLATE")
        .unwrap_or_else(|_| activity::DEFAULT_CHANNEL_NAME_TEMPLATE.to_owned());

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
            channel_locks: ChannelLocks::default(),
            default_channel_name_template,
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
