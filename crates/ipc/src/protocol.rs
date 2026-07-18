use serde::{Deserialize, Serialize};

/// Requests that the CLI client can send to the running bot daemon over the Unix socket.
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// Query daemon liveness and uptime.
    Status,
    /// Query guild and active-channel counts.
    Stats,
    /// Remove stale database entries for deleted Discord channels.
    Cleanup,
}

/// Responses the bot daemon sends back for each [`Request`] variant.
#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    /// Daemon is alive; contains uptime and whether Discord is reachable.
    Status { uptime_secs: u64, discord_ok: bool },
    /// Current guild and active temporary channel counts.
    Stats { guilds: u64, active_channels: u64 },
    /// Number of stale entries removed during cleanup.
    Cleanup { removed: u64 },
    /// An error occurred while processing the request.
    Error(String),
}
