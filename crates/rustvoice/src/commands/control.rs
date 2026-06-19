use ipc::{client::IpcClient, protocol::Request, protocol::Response};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn stats() -> Result<(), Error> {
    let mut client = IpcClient::connect(&socket_path()).await?;
    match client.send(Request::Stats).await? {
        Response::Stats { guilds, active_channels } => {
            println!("Guilds:          {guilds}");
            println!("Active channels: {active_channels}");
        }
        Response::Error(e) => eprintln!("Error: {e}"),
        _ => eprintln!("Unexpected response"),
    }
    Ok(())
}

pub async fn cleanup() -> Result<(), Error> {
    let mut client = IpcClient::connect(&socket_path()).await?;
    match client.send(Request::Cleanup).await? {
        Response::Cleanup { removed } => {
            println!("Removed {removed} dangling database entries.");
        }
        Response::Error(e) => eprintln!("Error: {e}"),
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
