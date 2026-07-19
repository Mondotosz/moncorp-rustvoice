mod limit;
mod privacy;
mod rename;

pub use limit::{limit, unlimit};
pub use privacy::{private, public};
pub use rename::rename;

use poise::serenity_prelude::ChannelId;

use crate::{ChannelLocks, Context, Error};

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

/// Held for the duration of an exclusive `/private`/`/public` operation on a channel;
/// releases the channel on drop so a later call can claim it again.
struct ChannelLockGuard<'a> {
    locks: &'a ChannelLocks,
    channel_id: ChannelId,
}

impl Drop for ChannelLockGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut set) = self.locks.lock() {
            set.remove(&self.channel_id);
        }
    }
}

/// Attempts to claim `channel_id` for an exclusive privacy-toggle operation. Returns
/// `None` if another `/private`/`/public` invocation on the same channel is already
/// in flight — callers should reply and bail out rather than proceed, to avoid racing
/// on companion-channel creation/deletion.
fn try_lock_channel(locks: &ChannelLocks, channel_id: ChannelId) -> Option<ChannelLockGuard<'_>> {
    let mut set = locks.lock().ok()?;
    set.insert(channel_id)
        .then(|| ChannelLockGuard { locks, channel_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_concurrent_lock_on_same_channel_is_rejected() {
        let locks = ChannelLocks::default();
        let channel = ChannelId::new(1);

        let first = try_lock_channel(&locks, channel);
        assert!(first.is_some(), "first lock attempt should succeed");

        let second = try_lock_channel(&locks, channel);
        assert!(
            second.is_none(),
            "a second concurrent lock on the same channel must be rejected"
        );
    }

    #[test]
    fn lock_is_released_when_guard_drops() {
        let locks = ChannelLocks::default();
        let channel = ChannelId::new(1);

        {
            let _guard = try_lock_channel(&locks, channel).expect("first lock should succeed");
        } // guard dropped here

        let second = try_lock_channel(&locks, channel);
        assert!(
            second.is_some(),
            "the channel should be lockable again after the guard is dropped"
        );
    }

    #[test]
    fn locks_on_different_channels_do_not_interfere() {
        let locks = ChannelLocks::default();

        let a = try_lock_channel(&locks, ChannelId::new(1));
        let b = try_lock_channel(&locks, ChannelId::new(2));

        assert!(a.is_some());
        assert!(b.is_some());
    }
}
