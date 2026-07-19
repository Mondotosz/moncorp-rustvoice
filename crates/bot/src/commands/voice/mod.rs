mod limit;
mod privacy;
mod rename;

pub use limit::{limit, unlimit};
pub use privacy::{private, public};
pub use rename::rename;

use poise::serenity_prelude::ChannelId;

use crate::{Context, Error};

/// Returns the caller's current dynamic (bot-managed) voice channel, or sends a
/// "not in a channel" reply and returns `None` if they aren't in one.
async fn require_temp_channel(ctx: Context<'_>) -> Result<Option<ChannelId>, Error> {
    let channel_id = ctx.guild().and_then(|g| {
        g.voice_states
            .get(&ctx.author().id)
            .and_then(|vs| vs.channel_id)
    });

    let Some(channel_id) = channel_id else {
        ctx.say("You are not in a dynamic voice channel.").await?;
        return Ok(None);
    };

    let is_temp =
        db::repositories::temporary_channel::exists(channel_id.get() as i64, &ctx.data().db)
            .await?;
    if !is_temp {
        ctx.say("You are not in a dynamic voice channel.").await?;
        return Ok(None);
    }

    Ok(Some(channel_id))
}
