use anyhow::Result;

pub async fn run() -> Result<()> {
    let token = std::env::var("DISCORD_TOKEN")?;
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./db.sqlite".into());
    let socket = ipc::default_socket_path();

    let db = db::connection::connect(&db_url).await?;
    bot::run(token, db, socket).await?;
    Ok(())
}
