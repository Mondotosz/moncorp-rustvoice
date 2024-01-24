use poise::serenity_prelude::{ChannelType, PermissionOverwrite};
use serenity::http::CacheHttp;
use serenity::model::Permissions;

use crate::models::{PrimaryChannel, TemporaryChannel};
use crate::{Context, Data, Error};

async fn get_voice_channel(
    ctx: Context<'_>,
) -> Result<serenity::model::channel::GuildChannel, Error> {
    let guild = ctx
        .guild()
        .ok_or_else(|| Error::from("Command invoked outside of a guild"))?;

    let mut cur_channel = None;

    for channel in guild.channels.values() {
        let channel = channel.clone().guild();

        let Some(channel) = channel else {continue;};

        if channel.kind != ChannelType::Voice {
            continue;
        }

        let cache = ctx.cache();

        let Ok(members) = channel.members(cache).await else {continue;};

        members.iter().find(|m| {
            if m.user.id == ctx.author().id {
                cur_channel = Some(channel.clone());
                return true;
            }

            false
        });
    }

    if cur_channel.is_none() {
        ctx.say("You must be in a voice channel to use this command.")
            .await?;
        return Err("User not found in any voice channel".into());
    }

    Ok(cur_channel.unwrap())
}

#[poise::command(
    slash_command,
    guild_only,
    required_permissions = "MANAGE_CHANNELS",
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn create(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Creating channel...").await?;

    if let Some(guild) = ctx.guild() {
        let channel = guild
            .create_channel(ctx.serenity_context().http(), |c| {
                c.name("➕ New Session").kind(ChannelType::Voice)
            })
            .await?;

        ctx.say(format!(
            "Created channel <#{}> ({})",
            channel.id, channel.id
        ))
        .await?;

        let id: i64 = channel.id.into();

        PrimaryChannel::insert(id, &ctx.data().db).await?;
    }

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn rename(
    ctx: Context<'_>,
    #[description = "The new name of the channel"] name: String,
) -> Result<(), Error> {
    let mut channel = get_voice_channel(ctx).await?;

    // Check if the channel is temporary before renaming
    if !TemporaryChannel::exists(channel.id.into(), &ctx.data().db).await? {
        ctx.say("Permanent channels cannot be renamed").await?;
        return Ok(());
    }

    let result = channel
        .edit(ctx.http(), |c| c.name(format!("[{}]", &name)))
        .await;

    match result {
        Ok(_) => {
            ctx.say(format!("Renamed channel to {}", &name)).await?;
            Ok(())
        }
        Err(_) => Err("Failed to rename channel".into()),
    }
}

#[poise::command(
    slash_command,
    guild_only,
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn private(ctx: Context<'_>) -> Result<(), Error> {
    let mut channel = get_voice_channel(ctx).await?;

    // Check if the channel is temporary before renaming
    if !TemporaryChannel::exists(channel.id.into(), &ctx.data().db).await? {
        ctx.say("Permanent channels cannot be modified").await?;
        return Ok(());
    }

    let guild = ctx
        .guild()
        .ok_or_else(|| Error::from("Cannot get the guild"))?;

    channel
        .edit(&ctx.http(), |c| {
            c.permissions(vec![
                PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::CONNECT,
                    kind: serenity::model::channel::PermissionOverwriteType::Role(
                        guild.id.0.into(),
                    ),
                },
                PermissionOverwrite {
                    allow: Permissions::CONNECT,
                    deny: Permissions::empty(),
                    kind: serenity::model::channel::PermissionOverwriteType::Member(
                        ctx.framework().bot_id,
                    ),
                },
            ])
        })
        .await?;

    ctx.say("Making channel private...").await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn public(ctx: Context<'_>) -> Result<(), Error> {
    let mut channel = get_voice_channel(ctx).await?;

    // Check if the channel is temporary before renaming
    if !TemporaryChannel::exists(channel.id.into(), &ctx.data().db).await? {
        ctx.say("Permanent channels cannot be modified").await?;
        return Ok(());
    }

    let Some(guild) = ctx.guild() else {
        return Err("Cannot get guild".into());
    };

    channel
        .edit(&ctx.http(), |c| {
            c.permissions(vec![PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::empty(),
                kind: serenity::model::channel::PermissionOverwriteType::Role(guild.id.0.into()),
            }])
        })
        .await?;

    ctx.say("Making channel public...").await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn limit(
    ctx: Context<'_>,
    #[description = "The number of users"]
    #[max = 99]
    #[min = 1]
    number: u64,
) -> Result<(), Error> {
    let mut channel = get_voice_channel(ctx).await?;

    // Check if the channel is temporary before renaming
    if !TemporaryChannel::exists(channel.id.into(), &ctx.data().db).await? {
        ctx.say("Permanent channels cannot be modified").await?;
        return Ok(());
    }

    if channel
        .edit(ctx.http(), |c| c.user_limit(number))
        .await
        .is_err()
    {
        return Err("Failed to limit channel".into());
    }

    ctx.say(format!("Limited channel to {} users", number))
        .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn unlimit(ctx: Context<'_>) -> Result<(), Error> {
    let mut channel = get_voice_channel(ctx).await?;

    // Check if the channel is temporary before renaming
    if !TemporaryChannel::exists(channel.id.into(), &ctx.data().db).await? {
        ctx.say("Permanent channels cannot be modified").await?;
        return Ok(());
    }

    if channel.edit(ctx.http(), |c| c.user_limit(0)).await.is_err() {
        return Err("Failed to limit channel".into());
    }

    ctx.say("Removed channel limit").await?;

    Ok(())
}

pub async fn update_channels(
    ctx: &serenity::client::Context,
    data: &Data,
    old: &Option<serenity::model::voice::VoiceState>,
    new: &serenity::model::voice::VoiceState,
) -> Result<(), Error> {
    // TODO: handle errors after running all the handlers
    let _result = voice_join_handler(ctx, data, new).await;

    if let Some(old) = old {
        let _result = voice_leave_handler(ctx, data, old).await;
    }

    Ok(())
}

async fn voice_join_handler(
    ctx: &serenity::client::Context,
    data: &Data,
    state: &serenity::model::voice::VoiceState,
) -> Result<(), Error> {
    let Some(id) = state.channel_id else {
        // Happens when the user leaves
        return Ok(());
    };

    if PrimaryChannel::exists(id.into(), &data.db).await? {
        handle_primary_channels(ctx, state, data).await?;
    }

    Ok(())
}

async fn voice_leave_handler(
    ctx: &serenity::client::Context,
    data: &Data,
    state: &serenity::model::voice::VoiceState,
) -> Result<(), Error> {
    let channel_id = state
        .channel_id
        .ok_or_else(|| Error::from("No channel id found"))?;

    if TemporaryChannel::exists(channel_id.into(), &data.db).await? {
        handle_temporary_channels(ctx, data, state).await?;
    }
    Ok(())
}

async fn handle_primary_channels(
    ctx: &serenity::client::Context,
    state: &serenity::model::voice::VoiceState,
    data: &Data,
) -> Result<(), Error> {
    let guild = state
        .guild_id
        .ok_or_else(|| Error::from("Cannot get guild"))?
        .to_guild_cached(&ctx.cache)
        .ok_or_else(|| Error::from("Cannot get cached guild"))?;

    // Get the category of the primary channel
    let category = state
        .channel_id
        .unwrap_or_default()
        .to_channel_cached(&ctx.cache)
        .ok_or_else(|| Error::from("Cannot get cached channel"))?
        .guild()
        .ok_or_else(|| Error::from("Cannot get guild channel"))?
        .parent_id;

    // Create a temporary channel and move the user to it
    let channel = guild
        .create_channel(ctx.http(), |c| {
            let builder = c.name("[General]").kind(ChannelType::Voice);

            match category {
                Some(category) => builder.category(category),
                None => builder,
            }
        })
        .await?;

    // Save the new channel to the database
    TemporaryChannel::insert(channel.id.into(), &data.db).await?;

    // Move the user to the new channel
    let _member = state
        .member
        .as_ref()
        .ok_or_else(|| Error::from("Cannot get user to move"))?
        .edit(&ctx.http, |m| m.voice_channel(channel.id))
        .await;

    Ok(())
}

async fn handle_temporary_channels(
    ctx: &serenity::client::Context,
    data: &Data,
    state: &serenity::model::voice::VoiceState,
) -> Result<(), Error> {
    let guild = state
        .guild_id
        .ok_or_else(|| Error::from("No guild id found"))?
        .to_guild_cached(&ctx.cache)
        .ok_or_else(|| Error::from("No guild found in cache"))?;

    let channel = guild
        .channels
        .get(
            &state
                .channel_id
                .ok_or_else(|| Error::from("Cannot find channel id"))?,
        )
        .ok_or_else(|| Error::from("Cannot find channel"))?;

    // Check the number of users
    let len = channel
        .clone()
        .guild()
        .ok_or_else(|| Error::from("Cannot get guild from the channel"))?
        .members(&ctx.cache)
        .await?
        .len();

    if len > 0 {
        return Ok(());
    }

    // Delete if empty
    channel.delete(&ctx.http).await?;

    // Update the database
    TemporaryChannel::delete(channel.id().into(), &data.db).await?;

    Ok(())
}
