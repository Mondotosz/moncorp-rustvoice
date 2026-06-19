use clap::{Parser, Subcommand};

use crate::commands;

#[derive(Parser)]
#[command(name = "rustvoice", about = "Dynamic voice channel bot for Discord")]
pub struct Cli {
    /// Increase log verbosity (-v WARN, -vv INFO, -vvv DEBUG, -vvvv TRACE)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Interactive environment setup (token, database, etc.)
    Setup {
        #[command(subcommand)]
        action: Option<SetupAction>,
    },
    /// Run the bot in the foreground
    Run,
    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Manage database migrations
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
    /// Register slash commands with Discord (guild-scoped = instant, global = up to 1 h)
    Register {
        /// Register in a specific guild ID instead of globally (overrides DISCORD_SERVER_ID)
        #[arg(long)]
        guild: Option<u64>,
        /// Force global registration even if DISCORD_SERVER_ID is set
        #[arg(long)]
        global: bool,
    },
    /// Print guild and channel statistics (requires a running daemon)
    Stats,
    /// Remove database entries for deleted Discord channels (requires a running daemon)
    Cleanup,
}

#[derive(Subcommand)]
pub enum SetupAction {
    /// Initialize or migrate the database only
    Db,
}

#[derive(Subcommand)]
pub enum DaemonAction {
    /// Start the bot as a background daemon
    Start,
    /// Stop the running daemon
    Stop,
    /// Show daemon status and uptime
    Status,
}

#[derive(Subcommand)]
pub enum DbAction {
    /// Show the status of all migrations (applied and pending)
    Status,
    /// Drop all tables and re-run all migrations from scratch
    Fresh,
    /// Roll back all migrations then re-apply them all
    Refresh,
    /// Roll back all applied migrations
    Reset,
    /// Apply pending migrations
    Up {
        /// Number of migrations to apply (default: all pending)
        #[arg(short = 'n', long)]
        num: Option<u32>,
    },
    /// Roll back migrations
    Down {
        /// Number of migrations to roll back (default: 1)
        #[arg(short = 'n', long, default_value = "1")]
        num: u32,
    },
}

impl Cli {
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.command {
            Command::Setup { action } => commands::setup::run(action).await,
            Command::Run => commands::run::run().await,
            Command::Daemon { action } => commands::daemon::run(action).await,
            Command::Db { action } => commands::db::run(action).await,
            Command::Register { guild, global } => commands::register::run(guild, global).await,
            Command::Stats => commands::control::stats().await,
            Command::Cleanup => commands::control::cleanup().await,
        }
    }
}

pub fn init_tracing(verbose: u8) {
    use tracing::Level;
    use tracing_subscriber::EnvFilter;

    let level = match verbose {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(level.into()),
        )
        .init();
}
