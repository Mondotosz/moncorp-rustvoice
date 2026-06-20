use poise::serenity_prelude::{self as serenity, Permissions};

use crate::{
    permissions::{self, Category},
    Context, Error,
};

/// Register a voice channel as an auto-voice trigger.
#[poise::command(slash_command, guild_only, check = "has_manage_channels")]
pub async fn init(
    ctx: Context<'_>,
    #[description = "Voice channel that triggers auto-voice channel creation"]
    #[channel_types("Voice")]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    if channel.kind != serenity::ChannelType::Voice {
        ctx.send(
            poise::CreateReply::default()
                .content("Please select a voice channel.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let channel_id = channel.id.get() as i64;

    db::repositories::guild::upsert(guild_id, &ctx.data().db).await?;
    db::repositories::primary_channel::insert(channel_id, guild_id, &ctx.data().db).await?;

    ctx.say(format!(
        "<#{}> is now an auto-voice trigger. Users who join it will get their own channel.",
        channel.id
    ))
    .await?;
    Ok(())
}

/// Show the bot's permission status in this guild.
#[poise::command(slash_command, guild_only, check = "has_manage_channels")]
pub async fn permissions(ctx: Context<'_>) -> Result<(), Error> {
    let bot_perms = {
        let guild = ctx
            .guild()
            .ok_or_else(|| Error::Other("Not in a guild".to_string()))?;
        let bot_id = ctx.serenity_context().cache.current_user().id;
        guild
            .members
            .get(&bot_id)
            .map(|m| guild.member_permissions(m))
            .unwrap_or(Permissions::empty())
    };

    let mut lines = vec!["**Bot Permission Status**".to_string()];
    for entry in permissions::ENTRIES {
        let has = bot_perms.contains(entry.permission);
        let icon = match (has, entry.category) {
            (true, _) => "🟢",
            (false, Category::Core) => "🔴",
            (false, Category::Privacy) => "🟠",
        };
        lines.push(format!("{icon} **{}** — {}", entry.name, entry.description));
    }

    let missing_count = permissions::ENTRIES
        .iter()
        .filter(|e| !bot_perms.contains(e.permission))
        .count();

    lines.push(String::new());
    if missing_count == 0 {
        lines.push("✅ All required permissions are granted.".to_string());
    } else {
        lines.push(format!(
            "⚠️ Missing {missing_count} permission(s). \
             Re-invite the bot with `rustvoice invite` or adjust its role in Server Settings."
        ));
    }

    ctx.send(
        poise::CreateReply::default()
            .content(lines.join("\n"))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

async fn has_manage_channels(ctx: Context<'_>) -> Result<bool, Error> {
    let Some(member) = ctx.author_member().await else {
        return Ok(false);
    };
    let permissions = member.permissions(ctx)?;
    Ok(permissions.manage_channels())
}
