type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn run() -> Result<(), Error> {
    let token = std::env::var("DISCORD_TOKEN")?;
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./db.sqlite".into());
    let socket = std::env::var("IPC_SOCKET_PATH").unwrap_or_else(|_| default_socket());

    let db = db::connection::connect(&db_url).await?;
    bot::run(token, db, socket).await?;
    Ok(())
}

fn default_socket() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{home}/.local/share/rustvoice/rustvoice.sock")
}
