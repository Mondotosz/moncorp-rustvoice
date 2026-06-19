use std::future::Future;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

use crate::protocol::{Request, Response};

pub async fn listen(socket_path: &str) -> std::io::Result<UnixListener> {
    let _ = std::fs::remove_file(socket_path);
    if let Some(parent) = std::path::Path::new(socket_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    UnixListener::bind(socket_path)
}

pub async fn handle_connections<F, Fut>(listener: UnixListener, handler: F) -> std::io::Result<()>
where
    F: Fn(Request) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Response> + Send,
{
    loop {
        let (stream, _) = listener.accept().await?;
        let handler = handler.clone();
        tokio::spawn(async move {
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            if reader.read_line(&mut line).await.is_err() {
                return;
            }
            let request: Request = match serde_json::from_str(line.trim()) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("IPC: malformed request: {e}");
                    return;
                }
            };
            let response = handler(request).await;
            if let Ok(mut json) = serde_json::to_string(&response) {
                json.push('\n');
                let _ = writer.write_all(json.as_bytes()).await;
            }
        });
    }
}
