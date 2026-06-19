use poise::serenity_prelude as serenity;

use crate::{Context, Error};

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

async fn has_manage_channels(ctx: Context<'_>) -> Result<bool, Error> {
    let Some(member) = ctx.author_member().await else {
        return Ok(false);
    };
    let permissions = member.permissions(ctx)?;
    Ok(permissions.manage_channels())
}
