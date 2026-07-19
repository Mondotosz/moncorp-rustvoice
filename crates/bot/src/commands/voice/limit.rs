use poise::serenity_prelude::Permissions;

use crate::{permissions::PermissionResultExt, Context, Error};

/// Set a user limit (1–99) on your dynamic voice channel.
#[poise::command(slash_command, guild_only)]
pub async fn limit(
    ctx: Context<'_>,
    #[description = "Maximum number of users (1–99)"]
    #[min = 1_u32]
    #[max = 99_u32]
    count: u32,
) -> Result<(), Error> {
    let Some(channel_id) = super::require_temp_channel(ctx).await? else {
        return Ok(());
    };
    channel_id
        .edit(
            ctx,
            poise::serenity_prelude::builder::EditChannel::new().user_limit(count),
        )
        .await
        .requires(&[Permissions::MANAGE_CHANNELS])?;
    ctx.say(format!("User limit set to **{count}**.")).await?;
    Ok(())
}

/// Remove the user limit from your dynamic voice channel.
#[poise::command(slash_command, guild_only)]
pub async fn unlimit(ctx: Context<'_>) -> Result<(), Error> {
    let Some(channel_id) = super::require_temp_channel(ctx).await? else {
        return Ok(());
    };
    channel_id
        .edit(
            ctx,
            poise::serenity_prelude::builder::EditChannel::new().user_limit(0),
        )
        .await
        .requires(&[Permissions::MANAGE_CHANNELS])?;
    ctx.say("User limit removed.").await?;
    Ok(())
}
