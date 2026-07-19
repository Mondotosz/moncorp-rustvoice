use poise::serenity_prelude::CreateEmbed;

use crate::{leveling, Context, Error};

/// Show this server's voice activity stats.
#[poise::command(slash_command, guild_only)]
pub async fn serverstats(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let db = &ctx.data().db;

    let active_channels = db::repositories::temporary_channel::count_by_guild(guild_id, db).await?;
    let triggers = db::repositories::primary_channel::list_by_guild(guild_id, db)
        .await?
        .len();
    let total_seconds =
        db::repositories::user_profile::total_voice_seconds_by_guild(guild_id, db).await?;
    let voice_time = if total_seconds == 0 {
        "—".to_string()
    } else {
        leveling::format_duration(total_seconds)
    };

    let embed = CreateEmbed::new()
        .title("Server Stats")
        .colour(0x5865F2u32)
        .field("Active temp channels", active_channels.to_string(), true)
        .field("Registered triggers", triggers.to_string(), true)
        .field("Total voice time logged", voice_time, true);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
