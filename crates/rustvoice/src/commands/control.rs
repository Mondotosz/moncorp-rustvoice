use ipc::{client::IpcClient, protocol::Request, protocol::Response};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn stats() -> Result<(), Error> {
    let mut client = IpcClient::connect(&ipc::default_socket_path()).await?;
    match client.send(Request::Stats).await? {
        Response::Stats {
            guilds,
            active_channels,
        } => {
            println!("Guilds:          {guilds}");
            println!("Active channels: {active_channels}");
        }
        Response::Error(e) => eprintln!("Error: {e}"),
        _ => eprintln!("Unexpected response"),
    }
    Ok(())
}

pub async fn cleanup() -> Result<(), Error> {
    let mut client = IpcClient::connect(&ipc::default_socket_path()).await?;
    match client.send(Request::Cleanup).await? {
        Response::Cleanup { removed } => {
            println!("Removed {removed} stale/empty temporary channel entries.");
        }
        Response::Error(e) => eprintln!("Error: {e}"),
        _ => eprintln!("Unexpected response"),
    }
    Ok(())
}
