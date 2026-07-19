use poise::serenity_prelude::Permissions;

use crate::{permissions::PermissionResultExt, Context, Error};

/// Rename your current dynamic voice channel.
#[poise::command(slash_command, guild_only)]
pub async fn rename(
    ctx: Context<'_>,
    #[description = "New channel name"] name: String,
) -> Result<(), Error> {
    let Some(channel_id) = super::require_temp_channel(ctx).await? else {
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
