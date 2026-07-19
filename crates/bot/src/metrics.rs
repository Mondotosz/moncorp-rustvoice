//! Prometheus metrics recording, gated behind the `metrics` Cargo feature.
//!
//! Every function here is always callable regardless of whether the feature is
//! enabled — callers never need their own `#[cfg(feature = "metrics")]`. When the
//! feature is disabled, every function is a no-op.

use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};

use db::DatabaseConnection;

use crate::BotContext;

/// Starts the `/metrics` HTTP listener at `addr`. No-op when the `metrics` feature
/// is disabled.
pub fn init(_addr: SocketAddr) {
    #[cfg(feature = "metrics")]
    match metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(_addr)
        .install()
    {
        Ok(()) => tracing::info!("Metrics: listening on http://{_addr}/metrics"),
        Err(e) => tracing::error!("Metrics: failed to start exporter: {e}"),
    }
}

/// Seeds the active-temp-channels gauge from the current DB count, so a bot restart
/// doesn't leave the gauge at zero while temp channels already exist.
pub async fn init_active_channels_gauge(_db: &DatabaseConnection) {
    #[cfg(feature = "metrics")]
    if let Ok(count) = db::repositories::temporary_channel::count_all(_db).await {
        metrics::gauge!("rustvoice_temp_channels_active").set(count as f64);
    }
}

pub fn temp_channel_created() {
    #[cfg(feature = "metrics")]
    {
        metrics::counter!("rustvoice_temp_channels_created_total").increment(1);
        metrics::gauge!("rustvoice_temp_channels_active").increment(1.0);
    }
}

pub fn temp_channel_deleted() {
    #[cfg(feature = "metrics")]
    metrics::gauge!("rustvoice_temp_channels_active").decrement(1.0);
}

pub fn xp_awarded(_amount: i64) {
    #[cfg(feature = "metrics")]
    metrics::counter!("rustvoice_xp_awarded_total").increment(_amount.max(0) as u64);
}

pub fn daily_bonus_awarded() {
    #[cfg(feature = "metrics")]
    metrics::counter!("rustvoice_daily_bonuses_total").increment(1);
}

/// Spawns a background task that periodically mirrors the shard connection state
/// into a gauge — the same "all shards Connected" signal `ipc_server`'s
/// `Request::Status` computes on demand, but pushed on a timer for scraping. No-op
/// when the `metrics` feature is disabled.
pub fn spawn_discord_status_poll(_bot_ctx: Arc<OnceLock<BotContext>>) {
    #[cfg(feature = "metrics")]
    tokio::spawn(async move {
        loop {
            if let Some(ctx) = _bot_ctx.get() {
                let connected = ctx.is_connected().await;
                metrics::gauge!("rustvoice_discord_connected").set(if connected {
                    1.0
                } else {
                    0.0
                });
            }
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });
}
