use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber;

mod daemon;
mod ipc;
mod player;
mod playlist;
mod cli;
mod tui;
mod theme;
#[cfg(not(target_os = "windows"))]
mod gui;

#[derive(Parser)]
#[command(name = "musicplayer")]
#[command(about = "Minimalist music player with GUI", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon in background (legacy CLI)
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Legacy CLI commands
    Play {
        path: Option<String>,
    },
    Pause,
    Stop,
    Next,
    Prev,
    Volume { level: u8 },
    Add { paths: Vec<String> },
    Status,
    Playlist,
    Clear,
    Tui,
}

#[derive(Subcommand)]
enum DaemonAction {
    Start,
    Stop,
    Status,
    Restart,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging (skip for TUI to avoid log interference)
    if !matches!(cli.command, Some(Commands::Tui)) {
        tracing_subscriber::fmt::init();
    }

    match cli.command {
        Some(Commands::Daemon { action }) => {
            match action {
                DaemonAction::Start => {
                    // For now, just run the daemon in foreground for testing
                    eprintln!("Starting daemon in foreground (use Ctrl+C to stop)...");
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(async {
                        daemon::start().await?;
                        Ok::<(), anyhow::Error>(())
                    })?;
                }
                DaemonAction::Stop => {
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(cli::stop_daemon())?;
                }
                DaemonAction::Status => {
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(cli::daemon_status())?;
                }
                DaemonAction::Restart => {
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(async {
                        cli::stop_daemon().await?;
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        // Note: restart will spawn a new daemonized process
                        std::process::Command::new(std::env::current_exe().unwrap())
                            .arg("daemon")
                            .arg("start")
                            .spawn()?;
                        Ok::<(), anyhow::Error>(())
                    })?;
                }
            }
        }
        Some(Commands::Play { path }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cli::send_command(ipc::Command::Play { path }))?;
        }
        Some(Commands::Pause) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cli::send_command(ipc::Command::Pause))?;
        }
        Some(Commands::Stop) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cli::send_command(ipc::Command::Stop))?;
        }
        Some(Commands::Next) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cli::send_command(ipc::Command::Next))?;
        }
        Some(Commands::Prev) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cli::send_command(ipc::Command::Previous))?;
        }
        Some(Commands::Volume { level }) => {
            let rt = tokio::runtime::Runtime::new()?;
            let level = level.min(100);
            rt.block_on(cli::send_command(ipc::Command::SetVolume { level }))?;
        }
        Some(Commands::Add { paths }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cli::send_command(ipc::Command::AddTracks { paths }))?;
        }
        Some(Commands::Status) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cli::show_status())?;
        }
        Some(Commands::Playlist) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cli::show_playlist())?;
        }
        Some(Commands::Clear) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cli::send_command(ipc::Command::ClearPlaylist))?;
        }
        Some(Commands::Tui) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(tui::run_tui())?;
        }
        None => {
            // Default: launch TUI on all platforms
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(tui::run_tui()).unwrap();
        }
    }

    Ok(())
}
