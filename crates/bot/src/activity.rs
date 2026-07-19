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

    if let Some((game, count)) = game_counts.into_iter().max_by_key(|(_, c)| *c) {
        if count * 2 >= total {
            return format!("[{game}]");
        }
    }

    "[General]".to_owned()
}
