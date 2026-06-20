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

/// Called once per guild on bot reconnect. Removes or deletes temporary channels that
/// either no longer exist on Discord, or exist but are empty (bot missed the leave events).
async fn startup_cleanup(
    ctx: &serenity::Context,
    guild: &serenity::Guild,
    data: &Data,
) -> Result<(), Error> {
    let guild_id = guild.id;
    let channels =
        db::repositories::temporary_channel::list_by_guild(guild_id.get() as i64, &data.db).await?;

    if channels.is_empty() {
        return Ok(());
    }

    let mut removed = 0u32;
    for channel in channels {
        let channel_id = serenity::ChannelId::new(channel.id as u64);

        match ctx.http.get_channel(channel_id).await {
            Err(_) => {
                // Channel was deleted while the bot was offline — remove DB row only.
                // Best-effort delete of any associated join channel.
                if let Some(join_id) = channel.join_channel_id {
                    let _ = serenity::ChannelId::new(join_id as u64).delete(ctx).await;
                }
                db::repositories::temporary_channel::delete(channel.id, &data.db).await?;
                removed += 1;
                tracing::debug!("Startup cleanup: removed stale DB entry for channel {channel_id}");
            }
            Ok(_) => {
                // Channel still exists — check whether it is empty.
                let has_members = guild
                    .voice_states
                    .values()
                    .any(|vs| vs.channel_id == Some(channel_id));

                if !has_members {
                    // Delete the join channel before the temp channel.
                    if let Some(join_id) = channel.join_channel_id {
                        let _ = serenity::ChannelId::new(join_id as u64).delete(ctx).await;
                    }
                    let _ = channel_id.delete(ctx).await;
                    db::repositories::temporary_channel::delete(channel.id, &data.db).await?;
                    removed += 1;
                    tracing::debug!(
                        "Startup cleanup: deleted empty temp channel {channel_id} in guild {guild_id}"
                    );
                }
            }
        }
    }

    if removed > 0 {
        tracing::info!(
            "Startup cleanup for guild {guild_id}: removed {removed} stale/empty temp channel(s)"
        );
    }

    // Remove voice sessions for users who are no longer in any voice channel.
    // Sessions for users still in voice are kept intact (joined_at preserved).
    let active_user_ids: Vec<i64> = guild
        .voice_states
        .keys()
        .map(|id| id.get() as i64)
        .collect();
    if let Err(e) = db::repositories::voice_session::delete_orphaned(
        guild_id.get() as i64,
        &active_user_ids,
        &data.db,
    )
    .await
    {
        tracing::warn!("Startup cleanup: voice session orphan removal failed for guild {guild_id}: {e}");
    }

    Ok(())
}
