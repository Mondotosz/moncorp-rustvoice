use db::DatabaseConnection;

#[derive(Copy, Clone)]
enum Kind {
    Level(u32),
    StreakDays(i64),
    VoiceHours(i64),
}

#[derive(Copy, Clone)]
pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    kind: Kind,
}

impl Achievement {
    /// Pure decision: has this achievement's threshold been met by the given stats?
    fn is_earned(&self, level: u32, total_voice_seconds: i64, streak: i64) -> bool {
        match self.kind {
            Kind::Level(threshold) => level >= threshold,
            Kind::StreakDays(threshold) => streak >= threshold,
            Kind::VoiceHours(hours) => total_voice_seconds >= hours * 3600,
        }
    }
}

const LEVEL_10: Achievement = Achievement {
    id: "level-10",
    name: "Level 10",
    emoji: "🥉",
    kind: Kind::Level(10),
};
const LEVEL_25: Achievement = Achievement {
    id: "level-25",
    name: "Level 25",
    emoji: "🥈",
    kind: Kind::Level(25),
};
const LEVEL_50: Achievement = Achievement {
    id: "level-50",
    name: "Level 50",
    emoji: "🥇",
    kind: Kind::Level(50),
};
const LEVEL_100: Achievement = Achievement {
    id: "level-100",
    name: "Level 100",
    emoji: "🏆",
    kind: Kind::Level(100),
};

const STREAK_7: Achievement = Achievement {
    id: "streak-7",
    name: "7-Day Streak",
    emoji: "🔥",
    kind: Kind::StreakDays(7),
};
const STREAK_30: Achievement = Achievement {
    id: "streak-30",
    name: "30-Day Streak",
    emoji: "🔥",
    kind: Kind::StreakDays(30),
};
const STREAK_100: Achievement = Achievement {
    id: "streak-100",
    name: "100-Day Streak",
    emoji: "🔥",
    kind: Kind::StreakDays(100),
};

const VOICE_10H: Achievement = Achievement {
    id: "voice-10h",
    name: "10 Hours in Voice",
    emoji: "🎧",
    kind: Kind::VoiceHours(10),
};
const VOICE_100H: Achievement = Achievement {
    id: "voice-100h",
    name: "100 Hours in Voice",
    emoji: "🎧",
    kind: Kind::VoiceHours(100),
};
const VOICE_500H: Achievement = Achievement {
    id: "voice-500h",
    name: "500 Hours in Voice",
    emoji: "🎧",
    kind: Kind::VoiceHours(500),
};

/// All defined achievements, in display order.
pub const ALL: &[Achievement] = &[
    LEVEL_10, LEVEL_25, LEVEL_50, LEVEL_100, STREAK_7, STREAK_30, STREAK_100, VOICE_10H,
    VOICE_100H, VOICE_500H,
];

/// Checks `profile`'s current stats against every achievement and permanently records
/// any newly crossed ones, returning those newly unlocked. Idempotent — already-unlocked
/// achievements are left untouched.
///
/// Must be called right after any event that can move `xp`, `total_voice_seconds`, or
/// `streak` (session-end XP award, daily bonus). This matters most for streak
/// achievements: `user_profiles.streak` resets to 0/1 on a missed daily window and
/// there's no "best streak ever" column, so an achievement only survives a later reset
/// because it was recorded here at the moment the threshold was actually crossed.
pub async fn check_and_unlock(
    user_id: i64,
    guild_id: i64,
    profile: &db::entities::user_profile::Model,
    now: i64,
    db: &DatabaseConnection,
) -> Vec<&'static Achievement> {
    let level = crate::leveling::level_from_xp(profile.xp);
    let mut newly_unlocked = Vec::new();

    for achievement in ALL {
        if !achievement.is_earned(level, profile.total_voice_seconds, profile.streak) {
            continue;
        }
        match db::repositories::user_achievement::unlock(user_id, guild_id, achievement.id, now, db)
            .await
        {
            Ok(true) => newly_unlocked.push(achievement),
            Ok(false) => {}
            Err(e) => tracing::warn!(
                "achievements: failed to record {} for user {user_id}: {e}",
                achievement.id
            ),
        }
    }

    newly_unlocked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_achievements_use_level_not_raw_xp() {
        assert!(LEVEL_10.is_earned(10, 0, 0));
        assert!(!LEVEL_10.is_earned(9, i64::MAX, 0));
    }

    #[test]
    fn streak_achievements_compare_against_current_streak() {
        assert!(STREAK_7.is_earned(1, 0, 7));
        assert!(!STREAK_7.is_earned(1, 0, 6));
    }

    #[test]
    fn voice_hour_achievements_convert_hours_to_seconds() {
        assert!(VOICE_10H.is_earned(1, 10 * 3600, 0));
        assert!(!VOICE_10H.is_earned(1, 10 * 3600 - 1, 0));
    }

    #[tokio::test]
    async fn check_and_unlock_only_reports_newly_crossed_thresholds() {
        let db = db::connection::connect_in_memory_for_tests().await.unwrap();
        db::repositories::guild::upsert(1, &db).await.unwrap();

        let profile = db::entities::user_profile::Model {
            user_id: 42,
            guild_id: 1,
            xp: crate::leveling::xp_for_level(10),
            total_voice_seconds: 0,
            last_daily_at: None,
            streak: 0,
        };

        let first = check_and_unlock(42, 1, &profile, 1_000, &db).await;
        assert_eq!(
            first.iter().map(|a| a.id).collect::<Vec<_>>(),
            vec!["level-10"]
        );

        // Calling again with the same stats should unlock nothing new.
        let second = check_and_unlock(42, 1, &profile, 2_000, &db).await;
        assert!(second.is_empty());
    }

    #[tokio::test]
    async fn check_and_unlock_records_multiple_categories_at_once() {
        let db = db::connection::connect_in_memory_for_tests().await.unwrap();
        db::repositories::guild::upsert(1, &db).await.unwrap();

        let profile = db::entities::user_profile::Model {
            user_id: 42,
            guild_id: 1,
            xp: crate::leveling::xp_for_level(10),
            total_voice_seconds: 10 * 3600,
            last_daily_at: None,
            streak: 7,
        };

        let unlocked = check_and_unlock(42, 1, &profile, 1_000, &db).await;
        let mut ids: Vec<&str> = unlocked.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["level-10", "streak-7", "voice-10h"]);
    }
}
