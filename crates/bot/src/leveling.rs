const BASE: f64 = 3600.0;
const GROWTH: f64 = 1.047;

fn cost_99_100() -> i64 {
    (BASE * GROWTH.powf(98.0)) as i64
}

/// Total XP required to reach level `n` from level 1.
pub fn xp_for_level(n: u32) -> i64 {
    if n <= 1 {
        return 0;
    }
    if n <= 100 {
        (BASE * (GROWTH.powf((n - 1) as f64) - 1.0) / (GROWTH - 1.0)) as i64
    } else {
        let base = xp_for_level(100);
        let c99 = cost_99_100();
        let m = (n - 100) as i64;
        // Arithmetic: each post-100 level costs c99 + k*86400 more than the previous.
        // Sum_{k=1}^{m} (c99 + k*86400) = m*c99 + 86400 * m*(m+1)/2
        base + m * c99 + 86400 * m * (m + 1) / 2
    }
}

/// Level corresponding to the given total XP (binary search).
pub fn level_from_xp(xp: i64) -> u32 {
    let mut lo = 1u32;
    let mut hi = 300u32;
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if xp_for_level(mid) <= xp {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

/// XP earned within the current level (above that level's threshold).
pub fn xp_in_level(xp: i64) -> i64 {
    xp - xp_for_level(level_from_xp(xp))
}

/// XP needed to advance from the current level to the next.
pub fn xp_to_next_level(xp: i64) -> i64 {
    let level = level_from_xp(xp);
    xp_for_level(level + 1) - xp_for_level(level)
}

pub fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        "<1m".to_string()
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        let h = seconds / 3600;
        let m = (seconds % 3600) / 60;
        if m == 0 {
            format!("{h}h")
        } else {
            format!("{h}h {m}m")
        }
    }
}

pub fn progress_bar(current: i64, total: i64, width: usize) -> String {
    let pct = if total == 0 {
        1.0f64
    } else {
        (current as f64 / total as f64).clamp(0.0, 1.0)
    };
    let filled = (pct * width as f64) as usize;
    let empty = width - filled;
    format!(
        "[{}{}] {:.1}%",
        "█".repeat(filled),
        "░".repeat(empty),
        pct * 100.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_1_is_zero_xp() {
        assert_eq!(xp_for_level(1), 0);
    }

    #[test]
    fn level_2_costs_one_hour() {
        assert_eq!(xp_for_level(2), 3600);
    }

    #[test]
    fn level_100_near_two_thousand_hours() {
        let xp = xp_for_level(100);
        let hours = xp / 3600;
        assert!(
            hours > 1800 && hours < 2200,
            "level 100 = {hours}h (expected ~2000h)"
        );
    }

    #[test]
    fn post_100_grows_slower() {
        let cost_99_100 = xp_for_level(100) - xp_for_level(99);
        let cost_100_101 = xp_for_level(101) - xp_for_level(100);
        let cost_101_102 = xp_for_level(102) - xp_for_level(101);
        assert!(
            cost_100_101 > cost_99_100,
            "post-100 should cost more than 99→100"
        );
        assert_eq!(
            cost_101_102 - cost_100_101,
            86400,
            "each post-100 level adds exactly 24h"
        );
    }

    #[test]
    fn level_from_xp_roundtrip() {
        for n in [1, 5, 10, 50, 99, 100, 101, 150] {
            let xp = xp_for_level(n);
            assert_eq!(
                level_from_xp(xp),
                n,
                "level_from_xp(xp_for_level({n})) should be {n}"
            );
        }
    }

    #[test]
    fn format_duration_examples() {
        assert_eq!(format_duration(30), "<1m");
        assert_eq!(format_duration(90), "1m");
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(3660), "1h 1m");
        assert_eq!(format_duration(7320), "2h 2m");
    }

    #[test]
    fn progress_bar_zero_total_is_fully_filled() {
        assert_eq!(progress_bar(0, 0, 10), "[██████████] 100.0%");
    }

    #[test]
    fn progress_bar_current_over_total_is_clamped() {
        assert_eq!(progress_bar(15, 10, 10), "[██████████] 100.0%");
    }

    #[test]
    fn progress_bar_partial_fill() {
        assert_eq!(progress_bar(5, 10, 10), "[█████░░░░░] 50.0%");
    }

    #[test]
    fn xp_in_level_is_zero_at_a_level_boundary() {
        let xp = xp_for_level(10);
        assert_eq!(xp_in_level(xp), 0);
    }

    #[test]
    fn xp_in_level_reflects_progress_past_the_boundary() {
        let xp = xp_for_level(10) + 100;
        assert_eq!(xp_in_level(xp), 100);
    }

    #[test]
    fn xp_to_next_level_matches_the_next_boundarys_cost() {
        let xp = xp_for_level(10);
        assert_eq!(xp_to_next_level(xp), xp_for_level(11) - xp_for_level(10));
    }
}
