pub mod auto_voice;

use crate::{Context, Error};

#[poise::command(slash_command, required_permissions = "SEND_MESSAGES")]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}
