use poise::serenity_prelude as serenity;

use crate::Data;

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub async fn handle_voice_transition(
    user_id: serenity::UserId,
    old_channel_id: Option<serenity::ChannelId>,
    new_channel_id: Option<serenity::ChannelId>,
    guild_id: serenity::GuildId,
    data: &Data,
) {
    // Ignore mute/deafen/stream events where the channel doesn't actually change.
    if old_channel_id == new_channel_id {
        return;
    }

    let uid = user_id.get() as i64;
    let gid = guild_id.get() as i64;
    let now = now_unix();

    // Close session if leaving a bot-managed temp channel.
    if let Some(old_id) = old_channel_id {
        match db::repositories::temporary_channel::exists(old_id.get() as i64, &data.db).await {
            Ok(true) => {
                match db::repositories::voice_session::end(uid, gid, &data.db).await {
                    Ok(Some(joined_at)) => {
                        let duration = (now - joined_at).max(0);
                        if duration > 0 {
                            if let Err(e) = db::repositories::user_profile::add_xp(
                                uid, gid, duration, duration, &data.db,
                            )
                            .await
                            {
                                tracing::warn!("XP: add_xp failed for user {uid} in guild {gid}: {e}");
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => tracing::warn!("XP: voice_session::end failed: {e}"),
                }
            }
            Ok(false) => {}
            Err(e) => tracing::warn!("XP: temp channel existence check failed: {e}"),
        }
    }

    // Open session if joining a bot-managed temp channel.
    if let Some(new_id) = new_channel_id {
        match db::repositories::temporary_channel::exists(new_id.get() as i64, &data.db).await {
            Ok(true) => {
                // Award daily bonus on first temp-channel join each 24h window.
                let profile = db::repositories::user_profile::get(uid, gid, &data.db)
                    .await
                    .ok()
                    .flatten();
                let daily_eligible = profile
                    .and_then(|p| p.last_daily_at)
                    .map(|t| now - t >= 86400)
                    .unwrap_or(true);

                if daily_eligible {
                    if let Err(e) =
                        db::repositories::user_profile::add_xp(uid, gid, 3600, 0, &data.db).await
                    {
                        tracing::warn!("XP: daily bonus add_xp failed: {e}");
                    } else if let Err(e) =
                        db::repositories::user_profile::set_last_daily(uid, gid, now, &data.db)
                            .await
                    {
                        tracing::warn!("XP: set_last_daily failed: {e}");
                    } else {
                        tracing::debug!("XP: daily bonus awarded to user {uid} in guild {gid}");
                    }
                }

                if let Err(e) =
                    db::repositories::voice_session::start(uid, gid, now, &data.db).await
                {
                    tracing::warn!("XP: voice_session::start failed: {e}");
                }
            }
            Ok(false) => {}
            Err(e) => tracing::warn!("XP: temp channel existence check failed: {e}"),
        }
    }
}
