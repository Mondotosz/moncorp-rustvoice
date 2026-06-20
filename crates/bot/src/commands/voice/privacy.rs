use poise::serenity_prelude::{
    self as serenity, ChannelType, PermissionOverwrite, PermissionOverwriteType, Permissions,
};

use crate::{permissions::PermissionResultExt, Context, Error};

/// Make your voice channel private. Creates a "[join ↑]" channel for join requests.
#[poise::command(slash_command, guild_only)]
pub async fn private(ctx: Context<'_>) -> Result<(), Error> {
    let Some(channel_id) = user_temp_channel(ctx).await? else {
        ctx.say("You are not in a dynamic voice channel.").await?;
        return Ok(());
    };

    let everyone_id = ctx.guild_id().unwrap().everyone_role();
    let bot_id = ctx.serenity_context().cache.current_user().id;

    // Grant the bot explicit access first so the subsequent @everyone deny cannot lock it out.
    // MANAGE_CHANNELS and MANAGE_ROLES are included so that category-level grants are preserved
    // on this channel after the overwrite is applied (member overwrites replace inherited ones).
    channel_id
        .create_permission(
            ctx,
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::CONNECT
                    | Permissions::MANAGE_CHANNELS
                    | Permissions::MANAGE_ROLES,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(bot_id),
            },
        )
        .await
        .requires(&[Permissions::MANAGE_ROLES])?;

    // Now deny @everyone from connecting to the channel.
    channel_id
        .create_permission(
            ctx,
            PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::CONNECT,
                kind: PermissionOverwriteType::Role(everyone_id),
            },
        )
        .await
        .requires(&[Permissions::MANAGE_ROLES])?;

    // Create the companion join-request channel in the same category.
    let parent_id = ctx
        .http()
        .get_channel(channel_id)
        .await
        .ok()
        .and_then(|c| c.guild())
        .and_then(|gc| gc.parent_id);

    let guild_id = ctx.guild_id().unwrap();
    let mut builder = serenity::builder::CreateChannel::new("[join ↑]").kind(ChannelType::Voice);
    if let Some(parent) = parent_id {
        builder = builder.category(parent);
    }
    // Explicitly allow @everyone to CONNECT so the join channel is reachable even if the
    // parent category has CONNECT denied.
    builder = builder.permissions(vec![PermissionOverwrite {
        allow: Permissions::CONNECT,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Role(everyone_id),
    }]);

    let join_ch = guild_id
        .create_channel(ctx, builder)
        .await
        .requires(&[Permissions::MANAGE_CHANNELS])?;

    db::repositories::temporary_channel::set_join_channel(
        channel_id.get() as i64,
        Some(join_ch.id.get() as i64),
        &ctx.data().db,
    )
    .await?;

    ctx.say("Channel is now private. Others can request to join via the \"[join ↑]\" channel.")
        .await?;
    Ok(())
}

/// Make your dynamic voice channel public (removes all role restrictions).
#[poise::command(slash_command, guild_only)]
pub async fn public(ctx: Context<'_>) -> Result<(), Error> {
    let Some(channel_id) = user_temp_channel(ctx).await? else {
        ctx.say("You are not in a dynamic voice channel.").await?;
        return Ok(());
    };

    // Delete the join-request channel if one exists.
    if let Some(record) =
        db::repositories::temporary_channel::find(channel_id.get() as i64, &ctx.data().db).await?
    {
        if let Some(join_id) = record.join_channel_id {
            let _ = serenity::ChannelId::new(join_id as u64).delete(ctx).await;
            db::repositories::temporary_channel::set_join_channel(
                channel_id.get() as i64,
                None,
                &ctx.data().db,
            )
            .await?;
        }
    }

    let everyone_id = ctx.guild_id().unwrap().everyone_role();

    // Remove the @everyone CONNECT deny. The bot's member overwrite is intentionally left in
    // place so it retains MANAGE_CHANNELS and MANAGE_ROLES on the channel even if those were
    // only granted at the category level.
    channel_id
        .delete_permission(ctx, PermissionOverwriteType::Role(everyone_id))
        .await
        .requires(&[Permissions::MANAGE_ROLES])?;

    ctx.say("Channel is now public.").await?;
    Ok(())
}

async fn user_temp_channel(ctx: Context<'_>) -> Result<Option<serenity::ChannelId>, Error> {
    let guild = ctx
        .guild()
        .ok_or_else(|| Error::Other("Not in a guild".to_string()))?
        .clone();
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
