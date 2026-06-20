use std::sync::{Arc, OnceLock};

use poise::serenity_prelude::{self as serenity, Permissions};

use crate::{permissions, BotContext, BotError, Data, Error};

/// Returns the full list of slash commands registered with the Discord framework.
pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        crate::commands::admin::init(),
        crate::commands::admin::permissions(),
        crate::commands::profile::profile(),
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
            let msg = if let BotError::Permission(perm_err) = &error {
                let bot_perms = bot_guild_permissions(&ctx).await;
                let missing: Vec<&str> = perm_err
                    .required
                    .iter()
                    .filter(|&&p| !bot_perms.contains(p))
                    .filter_map(|p| {
                        permissions::ENTRIES
                            .iter()
                            .find(|e| e.permission == *p)
                            .map(|e| e.name)
                    })
                    .collect();
                if missing.is_empty() {
                    "Missing Permissions — all expected permissions appear to be granted. \
                     Check channel-level overrides or contact a server admin."
                        .to_string()
                } else {
                    let base = format!(
                        "Missing Permissions: the bot needs **{}** to perform this action.",
                        missing.join(", ")
                    );
                    let manage_roles_missing =
                        perm_err.required.contains(&Permissions::MANAGE_ROLES)
                            && !bot_perms.contains(Permissions::MANAGE_ROLES);
                    if manage_roles_missing {
                        format!(
                            "{base} **Manage Roles** can be granted server-wide (in the bot's \
                             role) or at minimum on the voice channel category's permission \
                             overrides — the bot does not use it to manage server roles."
                        )
                    } else {
                        format!(
                            "{base} A server admin can re-invite the bot or grant the missing \
                             permissions."
                        )
                    }
                }
            } else {
                format!("Error: {error}")
            };
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

pub(crate) async fn bot_guild_permissions(ctx: &crate::Context<'_>) -> Permissions {
    let serenity_ctx = ctx.serenity_context();
    let bot_id = serenity_ctx.cache.current_user().id;
    let Some(guild_id) = ctx.guild_id() else {
        return Permissions::empty();
    };

    // Try cache first — resolve eagerly so the GuildRef is not held across an await.
    if let Some(perms) = ctx.guild().and_then(|guild| {
        guild
            .members
            .get(&bot_id)
            .map(|m| guild.member_permissions(m))
    }) {
        return perms;
    }

    // Cache miss — fetch member via HTTP then compute against the cached guild roles.
    let Ok(member) = guild_id.member(serenity_ctx, bot_id).await else {
        return Permissions::empty();
    };

    ctx.guild()
        .map(|guild| guild.member_permissions(&member))
        .unwrap_or(Permissions::empty())
}
