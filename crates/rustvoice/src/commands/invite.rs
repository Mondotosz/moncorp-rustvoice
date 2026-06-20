type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn run() -> Result<(), Error> {
    let token = std::env::var("DISCORD_TOKEN")?;
    let url = bot::invite_url(&token).await?;
    println!("Invite URL:\n{url}");
    Ok(())
}
