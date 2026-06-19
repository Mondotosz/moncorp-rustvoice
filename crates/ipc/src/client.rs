use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::protocol::{Request, Response};

pub struct IpcClient {
    read: BufReader<tokio::net::unix::OwnedReadHalf>,
    write: tokio::net::unix::OwnedWriteHalf,
}

impl IpcClient {
    pub async fn connect(socket_path: &str) -> std::io::Result<Self> {
        UnixStream::connect(socket_path).await.map_err(|e| {
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
        })
        .map(|stream| {
            let (read, write) = stream.into_split();
            Self { read: BufReader::new(read), write }
        })
    }

    pub async fn send(
        &mut self,
        request: Request,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let mut json = serde_json::to_string(&request)?;
        json.push('\n');
        self.write.write_all(json.as_bytes()).await?;

        let mut line = String::new();
        self.read.read_line(&mut line).await?;
        Ok(serde_json::from_str(line.trim())?)
    }
}
