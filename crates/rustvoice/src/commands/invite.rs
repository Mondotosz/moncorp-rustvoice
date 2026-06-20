use anyhow::Result;

pub async fn run() -> Result<()> {
    let token = std::env::var("DISCORD_TOKEN")?;
    let url = bot::invite_url(&token)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Invite URL:\n{url}");
    Ok(())
}
