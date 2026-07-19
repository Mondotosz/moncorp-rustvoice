use db::DatabaseConnection;

#[derive(Copy, Clone)]
enum Kind {
    Level(u32),
    StreakDays(i64),
    /// Longest *single* voice session, in hours — distinct from level, which already
    /// reflects cumulative voice time (plus daily-bonus XP) through the level curve.
    LongestSessionHours(i64),
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
    fn is_earned(&self, level: u32, longest_session_seconds: i64, streak: i64) -> bool {
        match self.kind {
            Kind::Level(threshold) => level >= threshold,
            Kind::StreakDays(threshold) => streak >= threshold,
            Kind::LongestSessionHours(hours) => longest_session_seconds >= hours * 3600,
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

const SESSION_2H: Achievement = Achievement {
    id: "session-2h",
    name: "2-Hour Session",
    emoji: "🎧",
    kind: Kind::LongestSessionHours(2),
};
const SESSION_6H: Achievement = Achievement {
    id: "session-6h",
    name: "6-Hour Session",
    emoji: "🎧",
    kind: Kind::LongestSessionHours(6),
};
const SESSION_12H: Achievement = Achievement {
    id: "session-12h",
    name: "12-Hour Session",
    emoji: "🎧",
    kind: Kind::LongestSessionHours(12),
};

/// All defined achievements, in display order.
pub const ALL: &[Achievement] = &[
    LEVEL_10,
    LEVEL_25,
    LEVEL_50,
    LEVEL_100,
    STREAK_7,
    STREAK_30,
    STREAK_100,
    SESSION_2H,
    SESSION_6H,
    SESSION_12H,
];

/// Checks `profile`'s current stats against every achievement and permanently records
/// any newly crossed ones, returning those newly unlocked. Idempotent — already-unlocked
/// achievements are left untouched.
///
/// Must be called right after any event that can move `xp`, `longest_session_seconds`,
/// or `streak` (session-end XP award, daily bonus). This matters most for streak
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

    // Skip achievements already recorded, so a maxed-out active user doesn't attempt
    // a redundant INSERT for every long-since-unlocked achievement on every event.
    let already_unlocked: std::collections::HashSet<String> =
        match db::repositories::user_achievement::list_by_user(user_id, guild_id, db).await {
            Ok(rows) => rows.into_iter().map(|r| r.achievement_id).collect(),
            Err(e) => {
                tracing::warn!("achievements: failed to list unlocked for user {user_id}: {e}");
                Default::default()
            }
        };

    for achievement in ALL {
        if already_unlocked.contains(achievement.id) {
            continue;
        }
        if !achievement.is_earned(level, profile.longest_session_seconds, profile.streak) {
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
    fn session_achievements_convert_hours_to_seconds() {
        assert!(SESSION_2H.is_earned(1, 2 * 3600, 0));
        assert!(!SESSION_2H.is_earned(1, 2 * 3600 - 1, 0));
    }

    #[test]
    fn session_achievements_ignore_level_and_streak() {
        // A long total voice time (reflected in level) should not itself satisfy a
        // longest-single-session threshold — only longest_session_seconds counts.
        assert!(!SESSION_2H.is_earned(100, 0, 999));
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
            longest_session_seconds: 0,
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
            total_voice_seconds: 0,
            last_daily_at: None,
            streak: 7,
            longest_session_seconds: 2 * 3600,
        };

        let unlocked = check_and_unlock(42, 1, &profile, 1_000, &db).await;
        let mut ids: Vec<&str> = unlocked.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["level-10", "session-2h", "streak-7"]);
    }
}
