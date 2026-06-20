use std::time::Duration;

use poise::serenity_prelude::{self as serenity, ChannelType, Context, Permissions};
use serenity::futures::StreamExt as _;

use crate::{permissions::PermissionResultExt, Data};

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
    // Check if this is a join-request channel.
    if let Some(temp_record) =
        db::repositories::temporary_channel::find_by_join_channel(channel_id.get() as i64, &data.db)
            .await?
    {
        handle_join_request(ctx, channel_id, temp_record, *user_id, guild_id).await?;
        return Ok(());
    }

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
    let temp_channel = builder
        .await
        .requires(&[Permissions::MANAGE_CHANNELS])?;

    db::repositories::temporary_channel::insert(
        temp_channel.id.get() as i64,
        guild_id.get() as i64,
        channel_id.get() as i64,
        &data.db,
    )
    .await?;

    // Move the user to the new channel
    guild_id
        .move_member(ctx, *user_id, temp_channel.id)
        .await
        .requires(&[Permissions::MOVE_MEMBERS])?;

    tracing::debug!(
        "Created temp channel {} for user {}",
        temp_channel.id,
        user_id
    );
    Ok(())
}

async fn handle_join_request(
    ctx: &Context,
    join_channel_id: serenity::ChannelId,
    temp_record: db::entities::temporary_channel::Model,
    requester_id: serenity::UserId,
    guild_id: serenity::GuildId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let private_channel_id = serenity::ChannelId::new(temp_record.id as u64);

    let allow_id = format!("join_allow:{}:{}", join_channel_id, requester_id);
    let deny_id = format!("join_deny:{}:{}", join_channel_id, requester_id);

    let msg = private_channel_id
        .send_message(
            ctx,
            serenity::builder::CreateMessage::new()
                .content(format!("<@{}> wants to join. Allow or deny?", requester_id))
                .components(vec![serenity::builder::CreateActionRow::Buttons(vec![
                    serenity::builder::CreateButton::new(&allow_id)
                        .label("Allow")
                        .style(serenity::ButtonStyle::Success),
                    serenity::builder::CreateButton::new(&deny_id)
                        .label("Deny")
                        .style(serenity::ButtonStyle::Danger),
                ])]),
        )
        .await
        .requires(&[Permissions::SEND_MESSAGES])?;

    // Spawn a task to collect the response without blocking the event handler.
    let ctx = ctx.clone();
    tokio::spawn(async move {
        await_join_response(
            ctx,
            msg,
            private_channel_id,
            join_channel_id,
            requester_id,
            guild_id,
        )
        .await;
    });

    Ok(())
}

async fn await_join_response(
    ctx: Context,
    mut msg: serenity::Message,
    private_channel_id: serenity::ChannelId,
    join_channel_id: serenity::ChannelId,
    requester_id: serenity::UserId,
    guild_id: serenity::GuildId,
) {
    let allow_id = format!("join_allow:{}:{}", join_channel_id, requester_id);
    let deny_id = format!("join_deny:{}:{}", join_channel_id, requester_id);
    let msg_id = msg.id;
    let mut stream = serenity::collector::ComponentInteractionCollector::new(&ctx)
        .filter(move |i| i.message.id == msg_id)
        .timeout(Duration::from_secs(120))
        .stream();

    loop {
        match stream.next().await {
            None => {
                // Timeout — edit message to reflect expiry.
                let _ = msg
                    .edit(
                        &ctx,
                        serenity::builder::EditMessage::new()
                            .content(format!(
                                "~~<@{}> wants to join.~~ Request expired.",
                                requester_id
                            ))
                            .components(vec![]),
                    )
                    .await;
                break;
            }
            Some(interaction) => {
                // Only members currently inside the private channel may respond.
                let in_channel = ctx
                    .cache
                    .guild(guild_id)
                    .map(|g| {
                        g.voice_states
                            .get(&interaction.user.id)
                            .and_then(|vs| vs.channel_id)
                            == Some(private_channel_id)
                    })
                    .unwrap_or(false);

                if !in_channel {
                    let _ = interaction
                        .create_response(
                            &ctx,
                            serenity::builder::CreateInteractionResponse::Message(
                                serenity::builder::CreateInteractionResponseMessage::new()
                                    .content("You must be inside the private channel to respond.")
                                    .ephemeral(true),
                            ),
                        )
                        .await;
                    continue;
                }

                if interaction.data.custom_id == allow_id {
                    let _ = guild_id
                        .move_member(&ctx, requester_id, private_channel_id)
                        .await;
                    let _ = interaction
                        .create_response(
                            &ctx,
                            serenity::builder::CreateInteractionResponse::UpdateMessage(
                                serenity::builder::CreateInteractionResponseMessage::new()
                                    .content(format!(
                                        "✅ <@{}> was allowed in by <@{}>.",
                                        requester_id, interaction.user.id
                                    ))
                                    .components(vec![]),
                            ),
                        )
                        .await;
                } else if interaction.data.custom_id == deny_id {
                    let _ = guild_id
                        .edit_member(
                            &ctx,
                            requester_id,
                            serenity::builder::EditMember::new().disconnect_member(),
                        )
                        .await;
                    let _ = interaction
                        .create_response(
                            &ctx,
                            serenity::builder::CreateInteractionResponse::UpdateMessage(
                                serenity::builder::CreateInteractionResponseMessage::new()
                                    .content(format!(
                                        "❌ <@{}> was denied by <@{}>.",
                                        requester_id, interaction.user.id
                                    ))
                                    .components(vec![]),
                            ),
                        )
                        .await;
                } else {
                    continue;
                }

                tracing::debug!(
                    "Join request via {} resolved for user {}",
                    join_channel_id,
                    requester_id
                );
                break;
            }
        }
    }
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
        // Also delete the join channel if one was created.
        if let Some(record) =
            db::repositories::temporary_channel::find(channel_id.get() as i64, &data.db).await?
        {
            if let Some(join_id) = record.join_channel_id {
                let _ = serenity::ChannelId::new(join_id as u64).delete(ctx).await;
            }
        }
        channel_id
            .delete(ctx)
            .await
            .requires(&[Permissions::MANAGE_CHANNELS])?;
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
            .await
            .requires(&[Permissions::MANAGE_CHANNELS])?;
    }

    Ok(())
}
