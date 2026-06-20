use anyhow::Result;

pub async fn run(guild_flag: Option<u64>, force_global: bool) -> Result<()> {
    let token = std::env::var("DISCORD_TOKEN")?;

    let guild_id = if force_global {
        None
    } else {
        guild_flag.or_else(|| {
            std::env::var("DISCORD_SERVER_ID")
                .ok()
                .and_then(|s| s.parse().ok())
        })
    };

    bot::register_commands(&token, guild_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    match guild_id {
        Some(id) => println!("Commands registered in guild {id}."),
        None => println!("Commands registered globally (may take up to 1 hour to propagate)."),
    }
    Ok(())
}
