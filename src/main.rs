#![warn(clippy::str_to_string)]

mod commands;
mod models;

use commands::auto_voice::update_channels;
use poise::serenity_prelude as serenity;
use std::{env::var, time::Duration};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    pub db: sqlx::SqlitePool,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename("db.sqlite")
                .create_if_missing(true),
        )
        .await
        .expect("Couldn't connect to SQLite database");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Couldn't run migrations");

    // FrameworkOptions contains all of poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands: vec![
            commands::ping(),
            commands::auto_voice::create(),
            commands::auto_voice::rename(),
            commands::auto_voice::private(),
            commands::auto_voice::public(),
            commands::auto_voice::limit(),
            commands::auto_voice::unlimit(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            edit_tracker: Some(poise::EditTracker::for_timespan(Duration::from_secs(3600))),
            additional_prefixes: vec![
                poise::Prefix::Literal("hey bot"),
                poise::Prefix::Literal("hey bot,"),
            ],
            ..Default::default()
        },
        /// The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        /// This code is run before every command
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        /// This code is run after a command if it was successful (returned Ok)
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        /// Enforce command checks even for owners (enforced by default)
        /// Set to true to bypass checks, which is useful for testing
        skip_checks_for_owners: false,
        event_handler: |_ctx, event: &poise::Event<'_>, _framework, _data| {
            Box::pin(async move {
                match event {
                    poise::Event::VoiceStateUpdate { old , new } => {
                        update_channels(_ctx,_data, old, new).await
                    }
                    _ => {
                        // println!("Unused event triggered: {:?}", event.name());
                        Ok(())
                    }
                }
            })
        },
        ..Default::default()
    };

    poise::Framework::builder()
        .token(
            var("DISCORD_TOKEN")
                .expect("Missing `DISCORD_TOKEN` env var, see README for more information."),
        )
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { db })
            })
        })
        .options(options)
        .intents(
            serenity::GatewayIntents::privileged()
                | serenity::GatewayIntents::GUILDS
                | serenity::GatewayIntents::GUILD_VOICE_STATES
                | serenity::GatewayIntents::GUILD_MESSAGES
                | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS,
        )
        .run()
        .await
        .unwrap();
}
