use clap::{Parser, Subcommand};
use log::{error, info, warn};
use std::env;
use std::time::SystemTime;
use voice_core::bot;
use voice_core::bot::Error;
use voice_core::db::{init_db, migrate_db, DB};

#[derive(Parser)]
#[command(version,about, long_about = None)]
struct Cli {
    #[arg(short, action = clap::ArgAction::Count, verbatim_doc_comment)]
    /// Specify verbosity
    ///   -v show warnings
    ///   -vv show info
    ///   -vvv show debug
    ///   -vvvv show trace
    verbose: u8,

    #[arg(short, long, default_value = "db.sqlite")]
    /// Specify the database file
    file: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    // Run the bot normally
    Run {},
}

fn setup_logger(verbose: u8) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(match verbose {
            1 => log::LevelFilter::Warn,
            2 => log::LevelFilter::Info,
            3 => log::LevelFilter::Debug,
            4 => log::LevelFilter::Trace,
            _ => log::LevelFilter::Error,
        })
        .chain(std::io::stdout())
        .apply()?;

    Ok(())
}

async fn run(db: DB) -> Result<(), Error> {
    let token = match env::var("DISCORD_TOKEN").map_err(|e| e.into()) {
        Ok(token) => token,
        Err(e) => {
            error!("Missing DISCORD_TOKEN");
            return Err(e);
        }
    };

    let mut bot = match bot::Bot::new(token, db).await {
        Ok(bot) => bot,
        Err(e) => {
            error!("Failed to create bot: {e}");
            return Err(e);
        }
    };

    bot.start().await?;

    println!("Connected");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    setup_logger(cli.verbose)?;

    info!("Loading .env");
    match dotenvy::dotenv() {
        Ok(_) => info!("Loaded .env"),
        Err(_) => warn!("Unable to load .env file, skipping"),
    };

    let db = init_db(&cli.file).await?;
    migrate_db(&db).await?;

    match &cli.command.unwrap_or(Commands::Run {}) {
        Commands::Run {} => run(db).await,
    }
}
