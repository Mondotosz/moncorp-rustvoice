use clap::Parser;

use cli::Cli;

mod cli;
mod commands;

#[tokio::main]
async fn main() {
    if let Err(e) = Cli::parse().run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
