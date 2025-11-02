use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tokio::sync::Mutex;
use std::sync::Arc;
// use tracing::{info, error};

use crate::ipc::{Command, IpcServer, PlayerStatus, PlaybackState, Response};
use crate::player::Player;
use crate::playlist::Playlist;

pub struct Daemon {
    player: Arc<Player>,
    playlist: Arc<Mutex<Playlist>>,
    ipc_server: IpcServer,
    last_manual_command: std::sync::Mutex<std::time::Instant>,
}

impl Daemon {
    pub async fn new() -> Result<Self> {
        let (player, _event_rx) = Player::new()?;
        let playlist = Arc::new(Mutex::new(Playlist::new()));
        let ipc_server = IpcServer::new().await?;

        Ok(Self {
            player: Arc::new(player),
            playlist,
            ipc_server,
            last_manual_command: std::sync::Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(10)), // Initialize to past
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // info!("Daemon started");

        let mut next_track_check = tokio::time::interval(tokio::time::Duration::from_millis(500));

        loop {
            // Accept incoming connections (non-blocking)
            tokio::select! {
                result = self.ipc_server.accept() => {
                    match result {
                        Ok(mut conn) => {
                            // Handle the connection
                            match conn.recv().await {
                                Ok(command) => {
                                    let response = self.handle_command(command).await;
                                    if let Err(e) = conn.send(response).await {
                                        error!("Failed to send response: {}", e);
                                    }
                                }
                        Err(e) => {
                            // error!("Failed to send response: {}", e);
                        }
                            }
                        }
                        Err(e) => {
                            // error!("Failed to accept connection: {}", e);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    }
                }
                _ = next_track_check.tick() => {
                    // Check if current track ended and play next (less frequently)
                    // Only auto-play if it's been at least 2 seconds since last manual command
                    let time_since_manual = self.last_manual_command.lock().unwrap().elapsed();
                    if time_since_manual > std::time::Duration::from_secs(2) &&
                       self.player.is_empty() && self.player.get_state() == PlaybackState::Playing {
                        if let Some(next_track) = self.playlist.lock().await.next() {
                            // info!("Auto-playing next track: {}", next_track);
                            if let Err(e) = self.player.load_track(next_track) {
                                // error!("Failed to load next track: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    async fn handle_command(&self, command: Command) -> Response {
        // Update last manual command timestamp for commands that change tracks
        match command {
            Command::Play { .. } | Command::Next | Command::Previous | Command::Stop => {
                *self.last_manual_command.lock().unwrap() = std::time::Instant::now();
            }
            _ => {}
        }

        match command {
            Command::Play { path } => {
                if let Some(path) = path {
                    // Play specific file
                    match self.player.load_track(path.clone()) {
                        Ok(_) => {
                            // info!("Playing: {}", path);
                            Response::Ok
                        }
                        Err(e) => Response::Error(format!("Failed to play: {}", e)),
                    }
                } else {
                    // Resume current track or start first track from playlist
                    if self.player.get_current_track().is_some() {
                        // Resume if there's a current track
                        match self.player.play() {
                        Ok(_) => {
                            // info!("Resumed playback");
                            Response::Ok
                        }
                            Err(e) => Response::Error(format!("Failed to resume: {}", e)),
                        }
                    } else {
                        // No current track - load first from playlist
                        let mut playlist = self.playlist.lock().await;
                        if playlist.is_empty() {
                            drop(playlist);
                            Response::Error("Playlist is empty".to_string())
                        } else {
                            // If no current track, start from beginning
                            // next() will return first track if current_index is None
                            if let Some(first_track) = playlist.current().or_else(|| playlist.next()) {
                                drop(playlist);
                                match self.player.load_track(first_track.clone()) {
                                    Ok(_) => {
                                        // info!("Playing first track: {}", first_track);
                                        Response::Ok
                                    }
                                    Err(e) => Response::Error(format!("Failed to play first track: {}", e)),
                                }
                            } else {
                                drop(playlist);
                                Response::Error("Failed to get track from playlist".to_string())
                            }
                        }
                    }
                }
            }
            Command::Pause => match self.player.pause() {
                Ok(_) => {
                    // info!("Paused");
                    Response::Ok
                }
                Err(e) => Response::Error(format!("Failed to pause: {}", e)),
            },
            Command::Stop => match self.player.stop() {
                Ok(_) => {
                    // info!("Stopped");
                    Response::Ok
                }
                Err(e) => Response::Error(format!("Failed to stop: {}", e)),
            },
            Command::Next => {
                let mut playlist = self.playlist.lock().await;
                // Try up to 5 tracks to find one that loads successfully
                for _ in 0..5 {
                    if let Some(next_track) = playlist.next() {
                        drop(playlist);
                        match self.player.load_track(next_track.clone()) {
                            Ok(_) => {
                                // info!("Playing next: {}", next_track);
                                return Response::Ok;
                            }
                            Err(e) => {
                                // error!("Failed to load track {}: {}, trying next", next_track, e);
                                playlist = self.playlist.lock().await;
                                continue;
                            }
                        }
                    } else {
                        drop(playlist);
                        break;
                    }
                }
                Response::Error("No playable next track found".to_string())
            }
            Command::Previous => {
                let mut playlist = self.playlist.lock().await;
                // Try up to 5 tracks to find one that loads successfully
                for _ in 0..5 {
                    if let Some(prev_track) = playlist.previous() {
                        drop(playlist);
                        match self.player.load_track(prev_track.clone()) {
                            Ok(_) => {
                                // info!("Playing previous: {}", prev_track);
                                return Response::Ok;
                            }
                            Err(e) => {
                                // error!("Failed to load track {}: {}, trying previous", prev_track, e);
                                playlist = self.playlist.lock().await;
                                continue;
                            }
                        }
                    } else {
                        drop(playlist);
                        break;
                    }
                }
                Response::Error("No playable previous track found".to_string())
            }
            Command::SetVolume { level } => match self.player.set_volume(level) {
                Ok(_) => {
                    // info!("Volume set to {}", level);
                    Response::Ok
                }
                Err(e) => Response::Error(format!("Failed to set volume: {}", e)),
            },
            Command::AddTracks { paths } => {
                let mut playlist = self.playlist.lock().await;
                match playlist.add_tracks(paths.clone()) {
                    Ok(_) => {
                        // info!("Added {} tracks", paths.len());
                        Response::Ok
                    }
                    Err(e) => Response::Error(format!("Failed to add tracks: {}", e)),
                }
            }
            Command::GetStatus => {
                let playlist = self.playlist.lock().await;
                let status = PlayerStatus {
                    state: self.player.get_state(),
                    current_track: self.player.get_current_track(),
                    position: self.player.get_position(),
                    duration: self.player.get_duration(),
                    volume: self.player.get_volume(),
                    playlist_length: playlist.len(),
                    current_index: playlist.current_index(),
                };
                Response::Status(status)
            }
            Command::GetPlaylist => {
                let playlist = self.playlist.lock().await;
                Response::Playlist(playlist.get_tracks())
            }
            Command::ClearPlaylist => {
                let mut playlist = self.playlist.lock().await;
                playlist.clear();
                // info!("Playlist cleared");
                Response::Ok
            }
            Command::Shutdown => {
                // info!("Shutting down daemon");
                std::process::exit(0);
            }
        }
    }
}

fn get_pid_file() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "musicplayer")
        .context("Failed to get project directories")?;
    
    let data_dir = dirs.data_dir();
    fs::create_dir_all(data_dir)?;
    
    Ok(data_dir.join("daemon.pid"))
}

pub async fn start() -> Result<()> {
    // Check if daemon is already running
    let pid_file = get_pid_file()?;

    if pid_file.exists() {
        let pid_str = fs::read_to_string(&pid_file)?;
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Check if process is still running
            if is_process_running(pid) {
                // info!("Daemon is already running (PID: {})", pid);
                return Ok(());
            }
        }
        // Remove stale PID file
        fs::remove_file(&pid_file)?;
    }

    // Write PID file
    let pid = std::process::id();
    fs::write(&pid_file, pid.to_string())?;

    // info!("Starting daemon (PID: {})...", pid);

    // Create and run daemon
    let mut daemon = Daemon::new().await?;

    // info!("Daemon started successfully");

    // Run the daemon in the foreground (this will block)
    if let Err(e) = daemon.run().await {
        // error!("Daemon error: {}", e);
        return Err(e);
    }

    Ok(())
}

fn is_process_running(pid: i32) -> bool {
    // Simple check on Unix systems
    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    #[cfg(not(unix))]
    {
        false
    }
}
