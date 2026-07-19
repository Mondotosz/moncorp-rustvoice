use std::sync::{Arc, OnceLock};

use poise::serenity_prelude as serenity;

use db::DatabaseConnection;

use crate::BotContext;
use ipc::protocol::{Request, Response};

pub async fn serve(
    socket_path: String,
    db: DatabaseConnection,
    start_time: std::time::Instant,
    bot_ctx: Arc<OnceLock<BotContext>>,
) {
    let listener = match ipc::server::listen(&socket_path).await {
        Ok(l) => {
            tracing::info!("IPC socket: {socket_path}");
            l
        }
        Err(e) => {
            tracing::error!("IPC server failed to bind: {e}");
            return;
        }
    };

    let _ = ipc::server::handle_connections(listener, move |request| {
        let db = db.clone();
        let bot_ctx = bot_ctx.clone();
        async move { handle(request, &db, start_time, &bot_ctx).await }
    })
    .await;
}

async fn handle(
    request: Request,
    db: &DatabaseConnection,
    start_time: std::time::Instant,
    bot_ctx: &Arc<OnceLock<BotContext>>,
) -> Response {
    match request {
        Request::Status => {
            let discord_ok = match bot_ctx.get() {
                None => false,
                Some(ctx) => {
                    let runners = ctx.shard_manager.runners.lock().await;
                    !runners.is_empty()
                        && runners
                            .values()
                            .all(|r| r.stage == serenity::gateway::ConnectionStage::Connected)
                }
            };
            Response::Status {
                uptime_secs: start_time.elapsed().as_secs(),
                discord_ok,
            }
        }
        Request::Stats => match stats(db).await {
            Ok((guilds, active_channels)) => Response::Stats {
                guilds,
                active_channels,
            },
            Err(e) => Response::Error(e.to_string()),
        },
        Request::Cleanup => match cleanup(db, bot_ctx).await {
            Ok(removed) => Response::Cleanup { removed },
            Err(e) => Response::Error(e.to_string()),
        },
    }
}

async fn stats(db: &DatabaseConnection) -> Result<(u64, u64), crate::Error> {
    let guilds = db::repositories::guild::count(db).await?;
    let active_channels = db::repositories::temporary_channel::count_all(db).await?;
    Ok((guilds, active_channels))
}

/// Removes temporary_channel rows for channels that no longer exist on Discord,
/// and deletes Discord channels that are currently empty (bot missed leave events).
async fn cleanup(
    db: &DatabaseConnection,
    bot_ctx: &Arc<OnceLock<BotContext>>,
) -> Result<u64, crate::Error> {
    let ctx = bot_ctx.get().ok_or_else(|| {
        crate::BotError::Other("Bot not ready yet — try again in a moment".to_string())
    })?;

    let channels = db::repositories::temporary_channel::list_all(db).await?;
    let mut removed = 0u64;

    for channel in channels {
        let channel_id = serenity::ChannelId::new(channel.id as u64);
        let guild_id = serenity::GuildId::new(channel.guild_id as u64);

        match ctx.http.get_channel(channel_id).await {
            Err(_) => {
                // Channel is gone from Discord — remove DB row.
                if let Some(join_id) = channel.join_channel_id {
                    let join_channel_id = serenity::ChannelId::new(join_id as u64);
                    let _ = ctx.http.delete_channel(join_channel_id, None).await;
                }
                db::repositories::temporary_channel::delete(channel.id, db).await?;
                crate::metrics::temp_channel_deleted();
                removed += 1;
                tracing::debug!("Cleanup: removed stale DB entry for channel {channel_id}");
            }
            Ok(_) => {
                // Channel still exists — delete if empty per the cache.
                let is_empty = ctx
                    .cache
                    .guild(guild_id)
                    .map(|g| {
                        !g.voice_states
                            .values()
                            .any(|vs| vs.channel_id == Some(channel_id))
                    })
                    .unwrap_or(false);

                if is_empty {
                    if let Some(join_id) = channel.join_channel_id {
                        let join_channel_id = serenity::ChannelId::new(join_id as u64);
                        let _ = ctx.http.delete_channel(join_channel_id, None).await;
                    }
                    let _ = ctx.http.delete_channel(channel_id, None).await;
                    db::repositories::temporary_channel::delete(channel.id, db).await?;
                    crate::metrics::temp_channel_deleted();
                    removed += 1;
                    tracing::debug!(
                        "Cleanup: deleted empty temp channel {channel_id} in guild {guild_id}"
                    );
                }
            }
        }
    }

    Ok(removed)
}
