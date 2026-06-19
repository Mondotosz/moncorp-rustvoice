use poise::serenity_prelude::{self as serenity, ChannelType, Context};

use crate::Data;

pub async fn handle(
    ctx: &Context,
    old: Option<serenity::VoiceState>,
    new: serenity::VoiceState,
    data: &Data,
) {
    let guild_id = match new.guild_id {
        Some(id) => id,
        None => return,
    };

    // User left a channel
    if let Some(ref old_state) = old {
        if let Some(left_channel) = old_state.channel_id {
            if let Err(e) = on_leave(ctx, left_channel, guild_id, data).await {
                tracing::error!("voice leave handler: {e}");
            }
        }
    }

    // User joined a channel
    if let Some(joined_channel) = new.channel_id {
        if old.as_ref().and_then(|o| o.channel_id) == Some(joined_channel) {
            return; // same channel, no change
        }
        if let Err(e) = on_join(ctx, joined_channel, guild_id, &new.user_id, data).await {
            tracing::error!("voice join handler: {e}");
        }
    }
}

async fn on_join(
    ctx: &Context,
    channel_id: serenity::ChannelId,
    guild_id: serenity::GuildId,
    user_id: &serenity::UserId,
    data: &Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let is_primary =
        db::repositories::primary_channel::exists(channel_id.get() as i64, &data.db).await?;
    if !is_primary {
        // Not a trigger channel; check if we should rename an existing temp channel
        recalculate_name(ctx, channel_id, guild_id, data).await?;
        return Ok(());
    }

    // Create a new temporary voice channel in the same category
    let parent_id = ctx
        .http
        .get_channel(channel_id)
        .await
        .ok()
        .and_then(|c| c.guild())
        .and_then(|gc| gc.parent_id);

    let mut builder = guild_id.create_channel(
        ctx,
        serenity::builder::CreateChannel::new("[General]").kind(ChannelType::Voice),
    );
    if let Some(parent) = parent_id {
        builder = guild_id.create_channel(
            ctx,
            serenity::builder::CreateChannel::new("[General]")
                .kind(ChannelType::Voice)
                .category(parent),
        );
    }
    let temp_channel = builder.await?;

    db::repositories::temporary_channel::insert(
        temp_channel.id.get() as i64,
        guild_id.get() as i64,
        channel_id.get() as i64,
        &data.db,
    )
    .await?;

    // Move the user to the new channel
    guild_id.move_member(ctx, *user_id, temp_channel.id).await?;

    tracing::debug!(
        "Created temp channel {} for user {}",
        temp_channel.id,
        user_id
    );
    Ok(())
}

async fn on_leave(
    ctx: &Context,
    channel_id: serenity::ChannelId,
    guild_id: serenity::GuildId,
    data: &Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let is_temp =
        db::repositories::temporary_channel::exists(channel_id.get() as i64, &data.db).await?;
    if !is_temp {
        return Ok(());
    }

    let guild = match ctx.cache.guild(guild_id) {
        Some(g) => g.clone(),
        None => return Ok(()),
    };

    let member_count = guild
        .voice_states
        .values()
        .filter(|vs| vs.channel_id == Some(channel_id))
        .count();

    if member_count == 0 {
        channel_id.delete(ctx).await?;
        db::repositories::temporary_channel::delete(channel_id.get() as i64, &data.db).await?;
        tracing::debug!("Deleted empty temp channel {}", channel_id);
    } else {
        recalculate_name(ctx, channel_id, guild_id, data).await?;
    }

    Ok(())
}

async fn recalculate_name(
    ctx: &Context,
    channel_id: serenity::ChannelId,
    guild_id: serenity::GuildId,
    data: &Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let is_temp =
        db::repositories::temporary_channel::exists(channel_id.get() as i64, &data.db).await?;
    if !is_temp {
        return Ok(());
    }

    let guild = match ctx.cache.guild(guild_id) {
        Some(g) => g.clone(),
        None => return Ok(()),
    };

    let members: Vec<_> = guild
        .voice_states
        .values()
        .filter(|vs| vs.channel_id == Some(channel_id))
        .filter_map(|vs| guild.members.get(&vs.user_id).cloned())
        .collect();

    let new_name = crate::activity::suggested_name(&members, ctx).await;

    let current_name = guild
        .channels
        .get(&channel_id)
        .map(|c| c.name.clone())
        .unwrap_or_default();

    if current_name != new_name {
        channel_id
            .edit(ctx, serenity::builder::EditChannel::new().name(&new_name))
            .await?;
    }

    Ok(())
}
