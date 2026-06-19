use poise::serenity_prelude as serenity;

use crate::{Data, Error};

mod voice_state;

pub async fn handle(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::VoiceStateUpdate { old, new } => {
            voice_state::handle(ctx, old.clone(), new.clone(), data).await;
        }
        _ => {}
    }
    Ok(())
}
