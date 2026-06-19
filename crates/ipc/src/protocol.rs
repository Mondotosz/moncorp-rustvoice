use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Status,
    Stats,
    Cleanup,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Status { uptime_secs: u64 },
    Stats { guilds: u64, active_channels: u64 },
    Cleanup { removed: u64 },
    Error(String),
}
