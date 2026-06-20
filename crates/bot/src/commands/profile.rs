use poise::serenity_prelude::{self as serenity, CreateEmbed, CreateEmbedAuthor};

use crate::{leveling, Context, Error};

/// Show your voice XP profile, or another user's.
#[poise::command(slash_command, guild_only)]
pub async fn profile(
    ctx: Context<'_>,
    #[description = "User to view (default: yourself)"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let target = user.as_ref().unwrap_or_else(|| ctx.author());
    let guild_id = ctx.guild_id().unwrap();

    let uid = target.id.get() as i64;
    let gid = guild_id.get() as i64;

    let profile = db::repositories::user_profile::get(uid, gid, &ctx.data().db).await?;
    let xp = profile.as_ref().map(|p| p.xp).unwrap_or(0);
    let total_seconds = profile.as_ref().map(|p| p.total_voice_seconds).unwrap_or(0);
    let streak = profile.as_ref().map(|p| p.streak).unwrap_or(0);

    let level = leveling::level_from_xp(xp);
    let xp_in_level = leveling::xp_in_level(xp);
    let xp_to_next = leveling::xp_to_next_level(xp);

    let member = guild_id
        .member(ctx.serenity_context(), target.id)
        .await
        .ok();
    let display_name = member
        .as_ref()
        .map(|m| m.display_name().to_string())
        .unwrap_or_else(|| target.name.clone());
    let avatar_url = target
        .avatar_url()
        .unwrap_or_else(|| target.default_avatar_url());

    let bar = leveling::progress_bar(xp_in_level, xp_to_next, 20);
    let xp_field = format!("{xp_in_level} / {xp_to_next}");
    let voice_field = if total_seconds == 0 {
        "—".to_string()
    } else {
        leveling::format_duration(total_seconds)
    };
    let streak_field = if streak == 0 {
        "—".to_string()
    } else {
        format!("🔥 {streak}")
    };

    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new(&display_name).icon_url(&avatar_url))
        .colour(0x5865F2u32)
        .field("Level", level.to_string(), true)
        .field("XP", xp_field, true)
        .field("Voice Time", voice_field, true)
        .field("Streak", streak_field, true)
        .field("Progress", bar, false);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
