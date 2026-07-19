use poise::serenity_prelude::{ActivityType, Context, Member};
use std::collections::HashMap;

use db::DatabaseConnection;

/// App-level fallback used when neither a guild override nor `DEFAULT_CHANNEL_NAME_TEMPLATE`
/// is set. `{game}` is substituted with the majority game or `"General"`.
pub const DEFAULT_CHANNEL_NAME_TEMPLATE: &str = "[{game}]";

/// Resolves the channel-name template for a guild: its DB override if one is set,
/// otherwise the app-level default (env var or [`DEFAULT_CHANNEL_NAME_TEMPLATE`]).
pub async fn resolve_template(
    guild_id: i64,
    db: &DatabaseConnection,
    default_template: &str,
) -> String {
    match db::repositories::guild::channel_name_template(guild_id, db).await {
        Ok(Some(template)) => template,
        _ => default_template.to_owned(),
    }
}

/// Renders `template` by substituting `{game}` with `game`. Templates without the
/// placeholder are returned unchanged, giving admins the option of a static name.
pub fn render_channel_name(template: &str, game: &str) -> String {
    if template.contains("{game}") {
        template.replace("{game}", game)
    } else {
        template.to_owned()
    }
}

/// Renders `template` using the majority game if ≥ 50 % of members share it, else `"General"`.
pub async fn suggested_name(members: &[Member], ctx: &Context, template: &str) -> String {
    let total = members.len();
    if total == 0 {
        return render_channel_name(template, "General");
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

    pick_name(&game_counts, total, template)
}

/// Pure decision: the majority game (≥ 50 % of `total` members) rendered into `template`,
/// else `"General"`. Split out from [`suggested_name`] so the threshold logic is testable
/// without a live Discord cache.
fn pick_name(game_counts: &HashMap<String, usize>, total: usize, template: &str) -> String {
    if let Some((game, count)) = game_counts.iter().max_by_key(|(_, c)| **c) {
        if count * 2 >= total {
            return render_channel_name(template, game);
        }
    }
    render_channel_name(template, "General")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_counts_default_to_general() {
        assert_eq!(
            pick_name(&HashMap::new(), 5, DEFAULT_CHANNEL_NAME_TEMPLATE),
            "[General]"
        );
    }

    #[test]
    fn majority_game_wins() {
        let mut counts = HashMap::new();
        counts.insert("Chess".to_owned(), 3);
        assert_eq!(
            pick_name(&counts, 5, DEFAULT_CHANNEL_NAME_TEMPLATE),
            "[Chess]"
        );
    }

    #[test]
    fn exact_half_counts_as_majority() {
        let mut counts = HashMap::new();
        counts.insert("Chess".to_owned(), 2);
        assert_eq!(
            pick_name(&counts, 4, DEFAULT_CHANNEL_NAME_TEMPLATE),
            "[Chess]"
        );
    }

    #[test]
    fn below_half_falls_back_to_general() {
        let mut counts = HashMap::new();
        counts.insert("Chess".to_owned(), 1);
        assert_eq!(
            pick_name(&counts, 4, DEFAULT_CHANNEL_NAME_TEMPLATE),
            "[General]"
        );
    }

    #[test]
    fn ties_pick_a_consistent_winner_without_panicking() {
        let mut counts = HashMap::new();
        counts.insert("Chess".to_owned(), 2);
        counts.insert("Go".to_owned(), 2);
        let name = pick_name(&counts, 4, DEFAULT_CHANNEL_NAME_TEMPLATE);
        assert!(name == "[Chess]" || name == "[Go]");
    }

    #[test]
    fn custom_template_is_used_for_majority_game() {
        let mut counts = HashMap::new();
        counts.insert("Chess".to_owned(), 3);
        assert_eq!(pick_name(&counts, 5, "🎮 {game}"), "🎮 Chess");
    }

    #[test]
    fn custom_template_is_used_for_general_fallback() {
        assert_eq!(pick_name(&HashMap::new(), 5, "🎮 {game}"), "🎮 General");
    }

    #[test]
    fn render_channel_name_substitutes_the_placeholder() {
        assert_eq!(render_channel_name("[{game}]", "Chess"), "[Chess]");
    }

    #[test]
    fn render_channel_name_without_placeholder_is_returned_unchanged() {
        assert_eq!(render_channel_name("Voice Chat", "Chess"), "Voice Chat");
    }

    #[test]
    fn render_channel_name_substitutes_every_occurrence() {
        assert_eq!(
            render_channel_name("{game} - {game}", "Chess"),
            "Chess - Chess"
        );
    }

    #[tokio::test]
    async fn resolve_template_falls_back_to_default_when_unset() {
        let db = db::connection::connect_in_memory_for_tests().await.unwrap();
        assert_eq!(resolve_template(1, &db, "🎮 {game}").await, "🎮 {game}");
    }

    #[tokio::test]
    async fn resolve_template_prefers_the_guild_override() {
        let db = db::connection::connect_in_memory_for_tests().await.unwrap();
        db::repositories::guild::set_channel_name_template(1, Some("[{game}]".to_string()), &db)
            .await
            .unwrap();
        assert_eq!(
            resolve_template(1, &db, "🎮 {game}").await,
            "[{game}]".to_string()
        );
    }
}
