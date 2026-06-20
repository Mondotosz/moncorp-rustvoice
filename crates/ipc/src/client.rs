use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::protocol::{Request, Response};

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Async Unix socket client for communicating with the running bot daemon.
pub struct IpcClient {
    read: BufReader<tokio::net::unix::OwnedReadHalf>,
    write: tokio::net::unix::OwnedWriteHalf,
}

impl IpcClient {
    /// Connect to the bot daemon's Unix socket at `socket_path`.
    pub async fn connect(socket_path: &str) -> Result<Self, IpcError> {
        let stream = UnixStream::connect(socket_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound
                || e.kind() == std::io::ErrorKind::ConnectionRefused
            {
                std::io::Error::new(
                    e.kind(),
                    format!("Bot is not running (socket not found at {socket_path})"),
                )
            } else {
                e
            }
        })?;
        let (read, write) = stream.into_split();
        Ok(Self {
            read: BufReader::new(read),
            write,
        })
    }

    /// Send a [`Request`] to the daemon and wait for its [`Response`].
    pub async fn send(&mut self, request: Request) -> Result<Response, IpcError> {
        let mut json = serde_json::to_string(&request)?;
        json.push('\n');
        self.write.write_all(json.as_bytes()).await?;

        let mut line = String::new();
        self.read.read_line(&mut line).await?;
        Ok(serde_json::from_str(line.trim())?)
    }
}
