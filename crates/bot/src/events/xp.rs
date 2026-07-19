use poise::serenity_prelude as serenity;

use crate::Data;

const MIN_SESSION_SECS: i64 = 60;
const DAILY_BONUS_XP: i64 = 3600;
// Daily window: eligible from 22 h after last award; in-window up to 26 h.
const DAILY_EARLY_SECS: i64 = 22 * 3600;
pub(crate) const DAILY_LATE_SECS: i64 = 26 * 3600;

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
                        } else {
                            crate::metrics::xp_awarded(duration);
                            if let Err(e) = db::repositories::user_profile::update_longest_session(
                                uid, gid, duration, &data.db,
                            )
                            .await
                            {
                                tracing::warn!(
                                    "XP: update_longest_session failed for user {uid} in guild {gid}: {e}"
                                );
                            }
                            check_achievements(uid, gid, now, data).await;
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
    crate::metrics::xp_awarded(DAILY_BONUS_XP);
    crate::metrics::daily_bonus_awarded();

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

    check_achievements(uid, gid, now, data).await;

    tracing::debug!("XP: daily bonus awarded to user {uid} in guild {gid} (streak {new_streak})");
}

/// Re-fetches the profile and checks it against every achievement threshold. Called
/// after any event that can move `xp`, `total_voice_seconds`, or `streak`.
async fn check_achievements(uid: i64, gid: i64, now: i64, data: &Data) {
    match db::repositories::user_profile::get(uid, gid, &data.db).await {
        Ok(Some(profile)) => {
            crate::achievements::check_and_unlock(uid, gid, &profile, now, &data.db).await;
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("XP: achievement check failed for user {uid} in guild {gid}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_data() -> Data {
        Data {
            db: db::connection::connect_in_memory_for_tests()
                .await
                .expect("connect in-memory test db"),
            start_time: std::time::Instant::now(),
            owner_id: None,
            channel_locks: Default::default(),
            default_channel_name_template: crate::activity::DEFAULT_CHANNEL_NAME_TEMPLATE
                .to_owned(),
        }
    }

    /// Registers `channel_id` as a bot-managed temporary voice channel in `guild_id`.
    async fn seed_temp_channel(data: &Data, channel_id: i64, guild_id: i64) {
        db::repositories::guild::upsert(guild_id, &data.db)
            .await
            .unwrap();
        db::repositories::primary_channel::insert(9_000, guild_id, &data.db)
            .await
            .unwrap();
        db::repositories::temporary_channel::insert(channel_id, guild_id, 9_000, &data.db)
            .await
            .unwrap();
    }

    mod daily_bonus {
        use super::*;

        #[tokio::test]
        async fn first_ever_join_awards_bonus_with_streak_one() {
            let data = test_data().await;
            db::repositories::guild::upsert(1, &data.db).await.unwrap();
            award_daily_bonus_if_eligible(1, 1, 1_000_000, &data).await;

            let profile = db::repositories::user_profile::get(1, 1, &data.db)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(profile.xp, DAILY_BONUS_XP);
            assert_eq!(profile.streak, 1);
            assert_eq!(profile.last_daily_at, Some(1_000_000));
        }

        #[tokio::test]
        async fn too_early_awards_nothing_and_leaves_state_untouched() {
            let data = test_data().await;
            db::repositories::guild::upsert(1, &data.db).await.unwrap();
            let last_daily = 1_000_000;
            db::repositories::user_profile::set_daily_state(1, 1, last_daily, 5, &data.db)
                .await
                .unwrap();

            let too_early = last_daily + DAILY_EARLY_SECS - 1;
            award_daily_bonus_if_eligible(1, 1, too_early, &data).await;

            let profile = db::repositories::user_profile::get(1, 1, &data.db)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(profile.xp, 0);
            assert_eq!(profile.streak, 5);
            assert_eq!(profile.last_daily_at, Some(last_daily));
        }

        #[tokio::test]
        async fn in_window_advances_anchor_by_exactly_24h_and_increments_streak() {
            let data = test_data().await;
            db::repositories::guild::upsert(1, &data.db).await.unwrap();
            let last_daily = 1_000_000;
            db::repositories::user_profile::set_daily_state(1, 1, last_daily, 5, &data.db)
                .await
                .unwrap();

            // Well within [22h, 26h] of the old anchor, but not equal to "now".
            let now = last_daily + DAILY_EARLY_SECS + 3_600;
            award_daily_bonus_if_eligible(1, 1, now, &data).await;

            let profile = db::repositories::user_profile::get(1, 1, &data.db)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(profile.xp, DAILY_BONUS_XP);
            assert_eq!(profile.streak, 6);
            // Anchored to the *old* timestamp + 24h, not to `now`.
            assert_eq!(profile.last_daily_at, Some(last_daily + 86_400));
        }

        #[tokio::test]
        async fn missed_window_resets_streak_and_anchors_to_now() {
            let data = test_data().await;
            db::repositories::guild::upsert(1, &data.db).await.unwrap();
            let last_daily = 1_000_000;
            db::repositories::user_profile::set_daily_state(1, 1, last_daily, 5, &data.db)
                .await
                .unwrap();

            let now = last_daily + DAILY_LATE_SECS + 1;
            award_daily_bonus_if_eligible(1, 1, now, &data).await;

            let profile = db::repositories::user_profile::get(1, 1, &data.db)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(profile.xp, DAILY_BONUS_XP);
            assert_eq!(profile.streak, 1);
            assert_eq!(profile.last_daily_at, Some(now));
        }
    }

    mod voice_transition {
        use super::*;
        use poise::serenity_prelude::{ChannelId, GuildId, UserId};

        #[tokio::test]
        async fn joining_a_temp_channel_starts_a_session() {
            let data = test_data().await;
            seed_temp_channel(&data, 100, 1).await;

            handle_voice_transition(
                UserId::new(42),
                None,
                Some(ChannelId::new(100)),
                GuildId::new(1),
                &data,
            )
            .await;

            let sessions = db::repositories::voice_session::list_by_guild(1, &data.db)
                .await
                .unwrap();
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].user_id, 42);
        }

        #[tokio::test]
        async fn joining_a_non_temp_channel_does_nothing() {
            let data = test_data().await;
            db::repositories::guild::upsert(1, &data.db).await.unwrap();

            handle_voice_transition(
                UserId::new(42),
                None,
                Some(ChannelId::new(999)), // never registered as a temp channel
                GuildId::new(1),
                &data,
            )
            .await;

            let sessions = db::repositories::voice_session::list_by_guild(1, &data.db)
                .await
                .unwrap();
            assert!(sessions.is_empty());
        }

        #[tokio::test]
        async fn quickly_leaving_a_temp_channel_closes_the_session_without_xp() {
            let data = test_data().await;
            seed_temp_channel(&data, 100, 1).await;

            handle_voice_transition(
                UserId::new(42),
                None,
                Some(ChannelId::new(100)),
                GuildId::new(1),
                &data,
            )
            .await;
            // A test executes far faster than MIN_SESSION_SECS, so this leave is always
            // "too short" and should award no XP.
            handle_voice_transition(
                UserId::new(42),
                Some(ChannelId::new(100)),
                None,
                GuildId::new(1),
                &data,
            )
            .await;

            let sessions = db::repositories::voice_session::list_by_guild(1, &data.db)
                .await
                .unwrap();
            assert!(sessions.is_empty(), "session should be closed on leave");

            // The join awards the (unrelated) daily bonus, so `xp` alone isn't a clean
            // signal here — `total_voice_seconds` is only ever touched by session-duration
            // XP, which a sub-minute session must not receive.
            let profile = db::repositories::user_profile::get(42, 1, &data.db)
                .await
                .unwrap();
            let voice_seconds = profile.map(|p| p.total_voice_seconds).unwrap_or(0);
            assert_eq!(voice_seconds, 0, "a sub-minute session should not award XP");
        }

        #[tokio::test]
        async fn a_long_session_updates_the_longest_session_record() {
            let data = test_data().await;
            seed_temp_channel(&data, 100, 1).await;

            // Seed a session that started well over 2h ago so the leave transition
            // below computes a duration long enough to cross the session-2h achievement.
            let joined_at = crate::time::now_unix() - 2 * 3600 - 60;
            db::repositories::voice_session::start(42, 1, joined_at, &data.db)
                .await
                .unwrap();

            handle_voice_transition(
                UserId::new(42),
                Some(ChannelId::new(100)),
                None,
                GuildId::new(1),
                &data,
            )
            .await;

            let profile = db::repositories::user_profile::get(42, 1, &data.db)
                .await
                .unwrap()
                .unwrap();
            assert!(profile.longest_session_seconds >= 2 * 3600);
        }

        #[tokio::test]
        async fn same_channel_transition_is_a_no_op() {
            let data = test_data().await;
            seed_temp_channel(&data, 100, 1).await;

            // e.g. mute/deafen toggles fire VoiceStateUpdate without changing channel.
            handle_voice_transition(
                UserId::new(42),
                Some(ChannelId::new(100)),
                Some(ChannelId::new(100)),
                GuildId::new(1),
                &data,
            )
            .await;

            let sessions = db::repositories::voice_session::list_by_guild(1, &data.db)
                .await
                .unwrap();
            assert!(sessions.is_empty());
        }
    }
}
