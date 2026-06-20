use std::collections::HashSet;

use poise::serenity_prelude as serenity;

use crate::{Data, Error};

mod voice_state;
mod xp;

pub async fn handle(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::VoiceStateUpdate { old, new } => {
            voice_state::handle(ctx, old.clone(), new.clone(), data).await;
        }
        // On reconnect (not a new guild join) clean up stale and empty temp channels.
        // At this point the guild's voice_states reflect the current Discord state.
        serenity::FullEvent::GuildCreate { guild, is_new } if *is_new != Some(true) => {
            if let Err(e) = startup_cleanup(ctx, guild, data).await {
                tracing::error!("Startup cleanup for guild {}: {e}", guild.id);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Called once per guild on bot reconnect. Removes or deletes stale/empty temp channels,
/// and awards XP for voice sessions that ended while the bot was offline.
async fn startup_cleanup(
    ctx: &serenity::Context,
    guild: &serenity::Guild,
    data: &Data,
) -> Result<(), Error> {
    let guild_id = guild.id;
    let gid = guild_id.get() as i64;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let channels = db::repositories::temporary_channel::list_by_guild(gid, &data.db).await?;

    // Track which temp channel Discord IDs still exist, for session recovery below.
    let mut live_temp_ids: HashSet<u64> = HashSet::new();

    let mut removed = 0u32;
    for channel in &channels {
        let channel_id = serenity::ChannelId::new(channel.id as u64);

        match ctx.http.get_channel(channel_id).await {
            Err(_) => {
                // Channel deleted while bot was offline — remove DB row only.
                if let Some(join_id) = channel.join_channel_id {
                    let _ = serenity::ChannelId::new(join_id as u64).delete(ctx).await;
                }
                db::repositories::temporary_channel::delete(channel.id, &data.db).await?;
                removed += 1;
                tracing::debug!("Startup cleanup: removed stale DB entry for channel {channel_id}");
            }
            Ok(_) => {
                let has_members = guild
                    .voice_states
                    .values()
                    .any(|vs| vs.channel_id == Some(channel_id));

                if !has_members {
                    if let Some(join_id) = channel.join_channel_id {
                        let _ = serenity::ChannelId::new(join_id as u64).delete(ctx).await;
                    }
                    let _ = channel_id.delete(ctx).await;
                    db::repositories::temporary_channel::delete(channel.id, &data.db).await?;
                    removed += 1;
                    tracing::debug!(
                        "Startup cleanup: deleted empty temp channel {channel_id} in guild {guild_id}"
                    );
                } else {
                    live_temp_ids.insert(channel.id as u64);
                }
            }
        }
    }

    if removed > 0 {
        tracing::info!(
            "Startup cleanup for guild {guild_id}: removed {removed} stale/empty temp channel(s)"
        );
    }

    // Award XP for sessions that ended while the bot was offline, then discard them.
    // Sessions belonging to users still in a live temp channel are preserved.
    let sessions = db::repositories::voice_session::list_by_guild(gid, &data.db).await?;

    if !sessions.is_empty() {
        // Users currently sitting in a temp channel — their session stays open.
        let users_in_temp: HashSet<i64> = guild
            .voice_states
            .values()
            .filter(|vs| {
                vs.channel_id
                    .map(|id| live_temp_ids.contains(&id.get()))
                    .unwrap_or(false)
            })
            .map(|vs| vs.user_id.get() as i64)
            .collect();

        const MAX_DOWNTIME_XP: i64 = 4 * 3600; // 4 h cap
        const MIN_SESSION_SECS: i64 = 60;

        let mut recovered = 0u32;
        for session in sessions {
            if users_in_temp.contains(&session.user_id) {
                continue;
            }
            // User left (or moved to a non-temp channel) while bot was offline.
            let elapsed = (now - session.joined_at).clamp(0, MAX_DOWNTIME_XP);
            if elapsed >= MIN_SESSION_SECS {
                if let Err(e) = db::repositories::user_profile::add_xp(
                    session.user_id,
                    gid,
                    elapsed,
                    elapsed,
                    &data.db,
                )
                .await
                {
                    tracing::warn!(
                        "Startup cleanup: add_xp failed for user {}: {e}",
                        session.user_id
                    );
                }
            }
            if let Err(e) =
                db::repositories::voice_session::end(session.user_id, gid, &data.db).await
            {
                tracing::warn!(
                    "Startup cleanup: end session failed for user {}: {e}",
                    session.user_id
                );
            }
            recovered += 1;
        }

        if recovered > 0 {
            tracing::info!(
                "Startup cleanup for guild {guild_id}: recovered XP for {recovered} offline session(s)"
            );
        }
    }

    Ok(())
}
