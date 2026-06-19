use poise::serenity_prelude::{
    self as serenity, PermissionOverwrite, PermissionOverwriteType, Permissions,
};

use crate::{Context, Error};

/// Make your dynamic voice channel private (blocks everyone from joining).
#[poise::command(slash_command, guild_only)]
pub async fn private(ctx: Context<'_>) -> Result<(), Error> {
    let Some(channel_id) = user_temp_channel(ctx).await? else {
        ctx.say("You are not in a dynamic voice channel.").await?;
        return Ok(());
    };
    let everyone_id = ctx.guild_id().unwrap().everyone_role();
    channel_id
        .create_permission(
            ctx,
            PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::CONNECT,
                kind: PermissionOverwriteType::Role(everyone_id),
            },
        )
        .await?;
    ctx.say("Channel is now private.").await?;
    Ok(())
}

/// Make your dynamic voice channel public (removes all role restrictions).
#[poise::command(slash_command, guild_only)]
pub async fn public(ctx: Context<'_>) -> Result<(), Error> {
    let Some(channel_id) = user_temp_channel(ctx).await? else {
        ctx.say("You are not in a dynamic voice channel.").await?;
        return Ok(());
    };
    let everyone_id = ctx.guild_id().unwrap().everyone_role();
    channel_id
        .delete_permission(ctx, PermissionOverwriteType::Role(everyone_id))
        .await?;
    ctx.say("Channel is now public.").await?;
    Ok(())
}

async fn user_temp_channel(ctx: Context<'_>) -> Result<Option<serenity::ChannelId>, Error> {
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
