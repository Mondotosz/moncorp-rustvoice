use crate::{Context, Error};

/// Set a user limit (1–99) on your dynamic voice channel.
#[poise::command(slash_command, guild_only)]
pub async fn limit(
    ctx: Context<'_>,
    #[description = "Maximum number of users (1–99)"]
    #[min = 1_u32]
    #[max = 99_u32]
    count: u32,
) -> Result<(), Error> {
    let Some(channel_id) = user_temp_channel(ctx).await? else {
        ctx.say("You are not in a dynamic voice channel.").await?;
        return Ok(());
    };
    channel_id
        .edit(
            ctx,
            poise::serenity_prelude::builder::EditChannel::new().user_limit(count),
        )
        .await?;
    ctx.say(format!("User limit set to **{count}**.")).await?;
    Ok(())
}

/// Remove the user limit from your dynamic voice channel.
#[poise::command(slash_command, guild_only)]
pub async fn unlimit(ctx: Context<'_>) -> Result<(), Error> {
    let Some(channel_id) = user_temp_channel(ctx).await? else {
        ctx.say("You are not in a dynamic voice channel.").await?;
        return Ok(());
    };
    channel_id
        .edit(
            ctx,
            poise::serenity_prelude::builder::EditChannel::new().user_limit(0),
        )
        .await?;
    ctx.say("User limit removed.").await?;
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
