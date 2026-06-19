use poise::serenity_prelude::{ActivityType, Context, Member};
use std::collections::HashMap;

/// Returns `"[GameName]"` if ≥ 50 % of members share the same game, else `"[General]"`.
pub async fn suggested_name(members: &[Member], ctx: &Context) -> String {
    let mut game_counts: HashMap<String, usize> = HashMap::new();

    for member in members {
        let Some(presences) = ctx
            .cache
            .guild(member.guild_id)
            .and_then(|g| g.presences.get(&member.user.id).cloned())
        else {
            continue;
        };

        for activity in &presences.activities {
            if activity.kind == ActivityType::Playing {
                *game_counts.entry(activity.name.clone()).or_insert(0) += 1;
            }
        }
    }

    let total = members.len();
    if total == 0 {
        return "[General]".to_owned();
    }

    if let Some((game, count)) = game_counts.into_iter().max_by_key(|(_, c)| *c) {
        if count * 2 >= total {
            return format!("[{game}]");
        }
    }

    "[General]".to_owned()
}
