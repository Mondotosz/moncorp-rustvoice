use std::sync::{Arc, OnceLock};

use poise::serenity_prelude as serenity;

use crate::{BotContext, Data, Error};

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        crate::commands::admin::init(),
        crate::commands::voice::rename(),
        crate::commands::voice::limit(),
        crate::commands::voice::unlimit(),
        crate::commands::voice::private(),
        crate::commands::voice::public(),
    ]
}

pub async fn build_and_run(
    token: String,
    data: Data,
    bot_ctx: Arc<OnceLock<BotContext>>,
) -> Result<(), Error> {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: all_commands(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(crate::events::handle(ctx, event, framework, data))
            },
            on_error: |err| Box::pin(on_error(err)),
            pre_command: |ctx| {
                Box::pin(async move {
                    tracing::debug!("Command: /{}", ctx.command().name);
                })
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                register_commands_on_startup(ctx, framework).await?;
                bot_ctx
                    .set(BotContext {
                        http: ctx.http.clone(),
                        cache: ctx.cache.clone(),
                    })
                    .ok();
                tracing::info!("Bot ready");
                Ok(data)
            })
        })
        .build();

    let intents = serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_VOICE_STATES
        | serenity::GatewayIntents::GUILD_PRESENCES
        | serenity::GatewayIntents::GUILD_MEMBERS;

    serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?
        .start()
        .await?;

    Ok(())
}

/// On startup, prefer guild-scoped registration (instant) when DISCORD_SERVER_ID is set,
/// and clear global commands so stale entries don't linger in the client UI.
/// Without DISCORD_SERVER_ID, fall back to global registration (up to 1 hour propagation).
async fn register_commands_on_startup(
    ctx: &serenity::Context,
    framework: &poise::Framework<Data, Error>,
) -> Result<(), serenity::Error> {
    let commands = &framework.options().commands;

    if let Some(guild_id) = std::env::var("DISCORD_SERVER_ID")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(serenity::GuildId::new)
    {
        poise::builtins::register_in_guild(ctx, commands, guild_id).await?;
        // Wipe any global commands left over from previous versions
        serenity::Command::set_global_commands(ctx, vec![]).await?;
        tracing::info!("Commands registered in guild {guild_id} (instant)");
    } else {
        poise::builtins::register_globally(ctx, commands).await?;
        tracing::info!("Commands registered globally");
    }
    Ok(())
}

async fn on_error(err: poise::FrameworkError<'_, Data, Error>) {
    match err {
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!("Command /{} failed: {error:?}", ctx.command().name);
            let msg = format!("Error: {error}");
            let _ = ctx
                .send(poise::CreateReply::default().content(msg).ephemeral(true))
                .await;
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx, .. } => {
            if let Some(ref e) = error {
                tracing::error!("Check for /{} errored: {e:?}", ctx.command().name);
            }
            let _ = ctx
                .send(
                    poise::CreateReply::default()
                        .content("You don't have permission to use this command.")
                        .ephemeral(true),
                )
                .await;
        }
        other => {
            tracing::error!("Framework error: {other}");
        }
    }
}
