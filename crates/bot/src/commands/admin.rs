use poise::serenity_prelude as serenity;

use crate::{
    context_ext::ContextExt,
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
    if !require_voice_channel(ctx, &channel).await? {
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
    let bot_perms = crate::client::bot_guild_permissions(&ctx).await;

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

    ctx.say_ephemeral(lines.join("\n")).await
}

/// Re-register slash commands globally with Discord.
#[poise::command(slash_command, guild_only, check = "is_owner")]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let commands = &ctx.framework().options().commands;
    let create_cmds = poise::builtins::create_application_commands(commands);
    serenity::Command::set_global_commands(ctx.serenity_context(), create_cmds).await?;
    ctx.say("Slash commands registered globally. Changes may take up to 1 hour to propagate.")
        .await?;
    Ok(())
}

/// List all auto-voice trigger channels configured in this server.
#[poise::command(slash_command, guild_only, check = "has_manage_channels")]
pub async fn triggers(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let channels =
        db::repositories::primary_channel::list_by_guild(guild_id, &ctx.data().db).await?;

    let content = if channels.is_empty() {
        "No trigger channels are configured. Use `/init` to add one.".to_string()
    } else {
        let lines: Vec<String> = channels
            .iter()
            .map(|c| format!("• <#{}>", c.id as u64))
            .collect();
        format!("**Auto-voice trigger channels:**\n{}", lines.join("\n"))
    };

    ctx.say_ephemeral(content).await
}

/// Remove a trigger channel from the auto-voice system (does not delete the Discord channel).
#[poise::command(
    slash_command,
    guild_only,
    rename = "remove-trigger",
    check = "has_manage_channels"
)]
pub async fn remove_trigger(
    ctx: Context<'_>,
    #[description = "The trigger channel to remove"]
    #[channel_types("Voice")]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    if !require_voice_channel(ctx, &channel).await? {
        return Ok(());
    }

    let channel_id = channel.id.get() as i64;

    if !db::repositories::primary_channel::exists(channel_id, &ctx.data().db).await? {
        ctx.say_ephemeral(format!(
            "<#{}> is not a registered trigger channel.",
            channel.id
        ))
        .await?;
        return Ok(());
    }

    let active =
        db::repositories::temporary_channel::list_by_primary_channel(channel_id, &ctx.data().db)
            .await?;

    if !active.is_empty() {
        const MAX_SHOWN: usize = 10;
        let shown = active.len().min(MAX_SHOWN);
        let mut mention_list: String = active[..shown]
            .iter()
            .map(|c| format!("<#{}>", c.id as u64))
            .collect::<Vec<_>>()
            .join(", ");
        if active.len() > MAX_SHOWN {
            mention_list.push_str(&format!(" and {} more", active.len() - MAX_SHOWN));
        }
        ctx.say_ephemeral(format!(
            "Cannot remove <#{}>: {} active temp channel(s) were created from it: {}\n\
             Wait for them to empty or run `rustvoice cleanup` first.",
            channel.id,
            active.len(),
            mention_list
        ))
        .await?;
        return Ok(());
    }

    db::repositories::primary_channel::delete(channel_id, &ctx.data().db).await?;

    ctx.say(format!(
        "<#{}> is no longer an auto-voice trigger.",
        channel.id
    ))
    .await?;
    Ok(())
}

/// Configure per-server settings.
#[poise::command(
    slash_command,
    guild_only,
    subcommands("channel_name"),
    check = "has_manage_channels"
)]
pub async fn config(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set the temp-channel naming template for this server. Must contain `{game}`.
#[poise::command(
    slash_command,
    guild_only,
    rename = "channel-name",
    check = "has_manage_channels"
)]
pub async fn channel_name(
    ctx: Context<'_>,
    #[description = "Template containing {game}, e.g. \"[{game}]\" or \"🎮 {game}\""]
    #[max_length = 100]
    template: String,
) -> Result<(), Error> {
    if !template.contains("{game}") {
        ctx.say_ephemeral("Template must contain `{game}`, e.g. `[{game}]`.")
            .await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap().get() as i64;
    db::repositories::guild::set_channel_name_template(
        guild_id,
        Some(template.clone()),
        &ctx.data().db,
    )
    .await?;

    ctx.say(format!(
        "Channel name template set to `{template}`. New and renamed temp channels will use it."
    ))
    .await?;
    Ok(())
}

/// Sends an ephemeral "please select a voice channel" reply and returns `false` if
/// `channel` is not a voice channel, `true` otherwise.
async fn require_voice_channel(
    ctx: Context<'_>,
    channel: &serenity::GuildChannel,
) -> Result<bool, Error> {
    if channel.kind != serenity::ChannelType::Voice {
        ctx.say_ephemeral("Please select a voice channel.").await?;
        return Ok(false);
    }
    Ok(true)
}

async fn is_owner(ctx: Context<'_>) -> Result<bool, Error> {
    Ok(ctx.data().owner_id == Some(ctx.author().id))
}

async fn has_manage_channels(ctx: Context<'_>) -> Result<bool, Error> {
    let author_id = ctx.author().id;
    let Some(guild) = ctx.guild() else {
        return Ok(false);
    };
    let Some(member) = guild.members.get(&author_id) else {
        return Ok(false);
    };
    let Some(channel) = guild.channels.get(&ctx.channel_id()) else {
        return Ok(false);
    };
    Ok(guild.user_permissions_in(channel, member).manage_channels())
}
