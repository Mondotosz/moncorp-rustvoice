use std::time::Duration;

use poise::serenity_prelude::{self as serenity, CreateEmbed, CreateEmbedFooter};
use serenity::futures::StreamExt as _;

use crate::{context_ext::ContextExt, leveling, Context, Error};

const PAGE_SIZE: usize = 10;

/// Show the top voice XP earners in this server.
#[poise::command(slash_command, guild_only)]
pub async fn ranking(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let gid = guild_id.get() as i64;

    let profiles = db::repositories::user_profile::list_top_by_guild(gid, &ctx.data().db).await?;

    if profiles.is_empty() {
        ctx.say_ephemeral("No one has earned XP yet. Join a voice channel to get started!")
            .await?;
        return Ok(());
    }

    let total_pages = profiles.len().div_ceil(PAGE_SIZE);
    let mut page = 0usize;

    let components = if total_pages > 1 {
        page_buttons(page, total_pages)
    } else {
        vec![]
    };

    let reply = ctx
        .send(
            poise::CreateReply::default()
                .embed(build_embed(&profiles, page, total_pages))
                .components(components),
        )
        .await?;

    if total_pages <= 1 {
        return Ok(());
    }

    let msg = reply.message().await?;
    let msg_id = msg.id;

    let mut stream =
        serenity::collector::ComponentInteractionCollector::new(ctx.serenity_context())
            .filter(move |i| i.message.id == msg_id)
            .timeout(Duration::from_secs(60))
            .stream();

    while let Some(interaction) = stream.next().await {
        match interaction.data.custom_id.as_str() {
            "rank_prev" => page = page.saturating_sub(1),
            "rank_next" => page = (page + 1).min(total_pages - 1),
            _ => continue,
        }
        let _ = interaction
            .create_response(
                ctx.serenity_context(),
                serenity::builder::CreateInteractionResponse::UpdateMessage(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(build_embed(&profiles, page, total_pages))
                        .components(page_buttons(page, total_pages)),
                ),
            )
            .await;
    }

    // Disable buttons after the collector times out.
    let _ = reply
        .edit(
            ctx,
            poise::CreateReply::default()
                .embed(build_embed(&profiles, page, total_pages))
                .components(page_buttons_disabled()),
        )
        .await;

    Ok(())
}

fn build_embed(
    profiles: &[db::entities::user_profile::Model],
    page: usize,
    total_pages: usize,
) -> CreateEmbed {
    let start = page * PAGE_SIZE;
    let description = profiles[start..(start + PAGE_SIZE).min(profiles.len())]
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let rank = start + i + 1;
            let medal = match rank {
                1 => "🥇",
                2 => "🥈",
                3 => "🥉",
                _ => "  ",
            };
            let level = leveling::level_from_xp(p.xp);
            let time = if p.total_voice_seconds == 0 {
                "—".to_string()
            } else {
                leveling::format_duration(p.total_voice_seconds)
            };
            format!(
                "{medal} **#{rank}** <@{}> — Lv.{level} · {time}",
                p.user_id as u64
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    CreateEmbed::new()
        .title("🏆 Voice Rankings")
        .description(description)
        .colour(0xFFD700u32)
        .footer(CreateEmbedFooter::new(format!(
            "Page {}/{} · {} members ranked",
            page + 1,
            total_pages,
            profiles.len()
        )))
}

fn page_buttons(page: usize, total_pages: usize) -> Vec<serenity::builder::CreateActionRow> {
    vec![serenity::builder::CreateActionRow::Buttons(vec![
        serenity::builder::CreateButton::new("rank_prev")
            .label("◀")
            .style(serenity::ButtonStyle::Secondary)
            .disabled(page == 0),
        serenity::builder::CreateButton::new("rank_next")
            .label("▶")
            .style(serenity::ButtonStyle::Secondary)
            .disabled(page + 1 >= total_pages),
    ])]
}

fn page_buttons_disabled() -> Vec<serenity::builder::CreateActionRow> {
    vec![serenity::builder::CreateActionRow::Buttons(vec![
        serenity::builder::CreateButton::new("rank_prev")
            .label("◀")
            .style(serenity::ButtonStyle::Secondary)
            .disabled(true),
        serenity::builder::CreateButton::new("rank_next")
            .label("▶")
            .style(serenity::ButtonStyle::Secondary)
            .disabled(true),
    ])]
}
