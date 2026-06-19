use clap::Parser;

use cli::{Cli, Command, DaemonAction};

mod cli;
mod commands;

fn main() {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    cli::init_tracing(cli.verbose);

    // `daemon start` must daemonize BEFORE the Tokio runtime is created.
    // Forking inside a running multi-threaded runtime is unsafe.
    if let Command::Daemon {
        action: DaemonAction::Start,
    } = &cli.command
    {
        let pid_path = ipc::default_pid_path();
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));

        if let Some(parent) = std::path::Path::new(&pid_path).parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Cannot create PID directory: {e}");
                std::process::exit(1);
            }
        }

        println!("Starting daemon… PID file: {pid_path}");

        if let Err(e) = daemonize::Daemonize::new()
            .pid_file(&pid_path)
            .working_directory(cwd)
            .start()
        {
            eprintln!("Failed to daemonize: {e}");
            std::process::exit(1);
        }

        // We are now the daemon child process.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime");
        if let Err(e) = rt.block_on(commands::run::run()) {
            tracing::error!("Daemon exited with error: {e}");
            std::process::exit(1);
        }
        return;
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");
    if let Err(e) = rt.block_on(cli.run()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
