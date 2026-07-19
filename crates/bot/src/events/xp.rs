use poise::serenity_prelude as serenity;

use crate::Data;

const MIN_SESSION_SECS: i64 = 60;
const DAILY_BONUS_XP: i64 = 3600;
// Daily window: eligible from 22 h after last award; in-window up to 26 h.
const DAILY_EARLY_SECS: i64 = 22 * 3600;
const DAILY_LATE_SECS: i64 = 26 * 3600;

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
    let now = crate::time::now_unix();

    // Close session if leaving a bot-managed temp channel.
    if let Some(old_id) = old_channel_id {
        match db::repositories::temporary_channel::exists(old_id.get() as i64, &data.db).await {
            Ok(true) => match db::repositories::voice_session::end(uid, gid, &data.db).await {
                Ok(Some(joined_at)) => {
                    let duration = (now - joined_at).max(0);
                    if duration >= MIN_SESSION_SECS {
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
            },
            Ok(false) => {}
            Err(e) => tracing::warn!("XP: temp channel existence check failed: {e}"),
        }
    }

    // Open session if joining a bot-managed temp channel.
    if let Some(new_id) = new_channel_id {
        match db::repositories::temporary_channel::exists(new_id.get() as i64, &data.db).await {
            Ok(true) => {
                award_daily_bonus_if_eligible(uid, gid, now, data).await;

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

async fn award_daily_bonus_if_eligible(uid: i64, gid: i64, now: i64, data: &Data) {
    let profile = match db::repositories::user_profile::get(uid, gid, &data.db).await {
        Err(e) => {
            tracing::warn!("XP: daily bonus check failed for user {uid} in guild {gid}: {e}");
            return;
        }
        Ok(p) => p,
    };

    // Determine new anchor timestamp and streak based on the ±2 h grace window.
    let (new_last_daily, new_streak) = match profile {
        None => (now, 1),
        Some(ref p) => match p.last_daily_at {
            None => (now, 1),
            Some(last_daily) => {
                let elapsed = now - last_daily;
                if elapsed < DAILY_EARLY_SECS {
                    return; // too early
                } else if elapsed <= DAILY_LATE_SECS {
                    // In-window: anchor to original cadence, preserve streak.
                    (last_daily + 86_400, p.streak + 1)
                } else {
                    // Missed window: reset streak, anchor to now.
                    (now, 1)
                }
            }
        },
    };

    if let Err(e) =
        db::repositories::user_profile::add_xp(uid, gid, DAILY_BONUS_XP, 0, &data.db).await
    {
        tracing::warn!("XP: daily bonus add_xp failed for user {uid}: {e}");
        return;
    }

    if let Err(e) = db::repositories::user_profile::set_daily_state(
        uid,
        gid,
        new_last_daily,
        new_streak,
        &data.db,
    )
    .await
    {
        tracing::warn!("XP: set_daily_state failed for user {uid}: {e}");
        return;
    }

    tracing::debug!("XP: daily bonus awarded to user {uid} in guild {gid} (streak {new_streak})");
}
