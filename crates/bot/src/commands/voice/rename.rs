use poise::serenity_prelude::Permissions;

use crate::{permissions::PermissionResultExt, Context, Error};

/// Rename your current dynamic voice channel.
#[poise::command(slash_command, guild_only)]
pub async fn rename(
    ctx: Context<'_>,
    #[description = "New channel name"] name: String,
) -> Result<(), Error> {
    let Some(channel_id) = user_temp_channel(ctx).await? else {
        ctx.say("You are not in a dynamic voice channel.").await?;
        return Ok(());
    };

    channel_id
        .edit(
            ctx,
            poise::serenity_prelude::builder::EditChannel::new().name(&name),
        )
        .await
        .requires(&[Permissions::MANAGE_CHANNELS])?;
    ctx.say(format!("Channel renamed to **{name}**.")).await?;
    Ok(())
}

async fn user_temp_channel(
    ctx: Context<'_>,
) -> Result<Option<poise::serenity_prelude::ChannelId>, Error> {
    let guild = ctx.guild().ok_or("Not in a guild")?.clone();
    let Some(voice_state) = guild.voice_states.get(&ctx.author().id) else {
        return Ok(None);
    };
    let Some(channel_id) = voice_state.channel_id else {
        return Ok(None);
    };
    let is_temp =
        db::repositories::temporary_channel::exists(channel_id.get() as i64, &ctx.data().db)
            .await?;
    Ok(is_temp.then_some(channel_id))
}
