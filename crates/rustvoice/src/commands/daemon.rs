use crate::cli::DaemonAction;
use ipc::{client::IpcClient, protocol::Request, protocol::Response};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn run(action: DaemonAction) -> Result<(), Error> {
    match action {
        DaemonAction::Start => unreachable!("daemon start is handled in main() before Tokio"),
        DaemonAction::Stop => stop().await,
        DaemonAction::Status => status().await,
    }
}

async fn stop() -> Result<(), Error> {
    let pid_path = ipc::default_pid_path();
    let pid_str = std::fs::read_to_string(&pid_path)
        .map_err(|_| format!("PID file not found at {pid_path}. Is the daemon running?"))?;
    let pid: i32 = pid_str
        .trim()
        .parse()
        .map_err(|_| format!("Invalid PID in {pid_path}"))?;

    let status = std::process::Command::new("kill")
        .arg(pid.to_string())
        .status()?;

    if status.success() {
        println!("Sent SIGTERM to daemon (PID {pid}).");
    } else {
        eprintln!("kill returned non-zero. Process {pid} may already be stopped.");
    }
    Ok(())
}

async fn status() -> Result<(), Error> {
    let mut client = IpcClient::connect(&ipc::default_socket_path()).await?;
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
