use crate::cli::DaemonAction;
use ipc::{client::IpcClient, protocol::Request, protocol::Response};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn run(action: DaemonAction) -> Result<(), Error> {
    match action {
        DaemonAction::Start => start().await,
        DaemonAction::Stop => stop().await,
        DaemonAction::Status => status().await,
    }
}

async fn start() -> Result<(), Error> {
    // TODO: use the `daemonize` crate to fork into the background, write a PID
    // file, then call commands::run::run() in the child process.
    Err("daemon start not yet implemented — use `rustvoice run` for foreground mode".into())
}

async fn stop() -> Result<(), Error> {
    // TODO: read PID file and send SIGTERM.
    Err("daemon stop not yet implemented".into())
}

async fn status() -> Result<(), Error> {
    let mut client = IpcClient::connect(&socket_path()).await?;
    match client.send(Request::Status).await? {
        Response::Status { uptime_secs } => {
            let h = uptime_secs / 3600;
            let m = (uptime_secs % 3600) / 60;
            let s = uptime_secs % 60;
            println!("Bot is running. Uptime: {h}h {m}m {s}s");
        }
        Response::Error(e) => eprintln!("Bot error: {e}"),
        _ => eprintln!("Unexpected response"),
    }
    Ok(())
}

fn socket_path() -> String {
    std::env::var("IPC_SOCKET_PATH").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/.local/share/rustvoice/rustvoice.sock")
    })
}
