use poise::serenity_prelude as serenity;

use db::DatabaseConnection;

pub mod activity;
pub mod client;
pub mod commands;
pub mod events;
pub mod ipc_server;

pub struct Data {
    pub db: DatabaseConnection,
    pub start_time: std::time::Instant,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub async fn run(token: String, db: DatabaseConnection, socket_path: String) -> Result<(), Error> {
    let start_time = std::time::Instant::now();

    let ipc_db = db.clone();
    tokio::spawn(ipc_server::serve(socket_path, ipc_db, start_time));

    client::build_and_run(token, Data { db, start_time }).await
}

/// Register slash commands without starting the full bot.
/// Uses guild registration (instant) when `guild_id` is supplied, global otherwise.
pub async fn register_commands(token: &str, guild_id: Option<u64>) -> Result<(), Error> {
    let http = serenity::Http::new(token);
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
