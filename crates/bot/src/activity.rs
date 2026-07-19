use poise::serenity_prelude::{ActivityType, Context, Member};
use std::collections::HashMap;

/// Returns `"[GameName]"` if ≥ 50 % of members share the same game, else `"[General]"`.
pub async fn suggested_name(members: &[Member], ctx: &Context) -> String {
    let total = members.len();
    if total == 0 {
        return "[General]".to_owned();
    }

    let mut game_counts: HashMap<String, usize> = HashMap::new();

    // All members belong to the same guild, so the presence cache only needs fetching once.
    if let Some(guild) = members.first().and_then(|m| ctx.cache.guild(m.guild_id)) {
        for member in members {
            let Some(presence) = guild.presences.get(&member.user.id) else {
                continue;
            };
            for activity in &presence.activities {
                if activity.kind == ActivityType::Playing {
                    *game_counts.entry(activity.name.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    pick_name(&game_counts, total)
}

/// Pure decision: `"[GameName]"` if some game covers ≥ 50 % of `total` members, else
/// `"[General]"`. Split out from [`suggested_name`] so the threshold logic is testable
/// without a live Discord cache.
fn pick_name(game_counts: &HashMap<String, usize>, total: usize) -> String {
    if let Some((game, count)) = game_counts.iter().max_by_key(|(_, c)| **c) {
        if count * 2 >= total {
            return format!("[{game}]");
        }
    }
    "[General]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_counts_default_to_general() {
        assert_eq!(pick_name(&HashMap::new(), 5), "[General]");
    }

    #[test]
    fn majority_game_wins() {
        let mut counts = HashMap::new();
        counts.insert("Chess".to_owned(), 3);
        assert_eq!(pick_name(&counts, 5), "[Chess]");
    }

    #[test]
    fn exact_half_counts_as_majority() {
        let mut counts = HashMap::new();
        counts.insert("Chess".to_owned(), 2);
        assert_eq!(pick_name(&counts, 4), "[Chess]");
    }

    #[test]
    fn below_half_falls_back_to_general() {
        let mut counts = HashMap::new();
        counts.insert("Chess".to_owned(), 1);
        assert_eq!(pick_name(&counts, 4), "[General]");
    }

    #[test]
    fn ties_pick_a_consistent_winner_without_panicking() {
        let mut counts = HashMap::new();
        counts.insert("Chess".to_owned(), 2);
        counts.insert("Go".to_owned(), 2);
        let name = pick_name(&counts, 4);
        assert!(name == "[Chess]" || name == "[Go]");
    }
}
