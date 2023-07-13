use poise::serenity_prelude::ChannelType;
use serenity::http::CacheHttp;

use crate::{Context, Error};

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

        sqlx::query!("INSERT INTO primary_channels (id) VALUES (?)", id)
            .execute(&ctx.data().db)
            .await?;
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
    // println!("{:#?}", ctx.guild().unwrap().);

    // Get the voice channel of the user
    let guild = ctx.guild();

    if guild.is_none() {
        return Err("You must be in a guild to use this command".into());
    }

    let guild = guild.unwrap();
    let mut cur_channel = None;

    for channel in guild.channels.values() {
        let channel = channel.clone().guild();

        if channel.is_none() {
            continue;
        }

        let channel = channel.unwrap();

        if channel.kind != ChannelType::Voice {
            continue;
        }

        let cache = ctx.cache();

        if cache.is_none() {
            continue;
        }

        let members = channel.members(cache.unwrap()).await;

        if members.is_err() {
            continue;
        }

        members.unwrap().iter().find(|m| {
            if m.user.id == ctx.author().id {
                cur_channel = Some(channel.clone());
                return true;
            }

            false
        });
    }

    if cur_channel.is_none() {
        return Err("You must be in a voice channel to use this command".into());
    }

    // TODO: Check that the channel is a temporary channel before renaming

    let result = cur_channel
        .unwrap()
        .edit(ctx.http(), |c| c.name(&name))
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
    ctx.say("Making channel private...").await?;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn public(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Making channel public...").await?;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn limit(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Limiting channel...").await?;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn unlimit(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Unlimiting channel...").await?;
    Ok(())
}
