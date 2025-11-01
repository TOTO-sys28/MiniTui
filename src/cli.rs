use anyhow::{Context, Result};
use std::fs;

use crate::ipc::{Command, IpcClient, Response, PlaybackState};

pub async fn send_command(command: Command) -> Result<()> {
    match IpcClient::send_command(command).await {
        Ok(Response::Ok) => {
            println!("? Command executed successfully");
            Ok(())
        }
        Ok(Response::Error(e)) => {
            eprintln!("? Error: {}", e);
            std::process::exit(1);
        }
        Ok(_) => {
            eprintln!("? Unexpected response");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("? Failed to communicate with daemon: {}", e);
            eprintln!("  Make sure the daemon is running: musicplayer daemon start");
            std::process::exit(1);
        }
    }
}

pub async fn show_status() -> Result<()> {
    match IpcClient::send_command(Command::GetStatus).await {
        Ok(Response::Status(status)) => {
            println!("??????????????????????????????????????????");
            println!("?          Music Player Status           ?");
            println!("??????????????????????????????????????????");
            println!();
            
            let state_emoji = match status.state {
                PlaybackState::Playing => "?",
                PlaybackState::Paused => "?",
                PlaybackState::Stopped => "?",
            };
            
            let state_str = match status.state {
                PlaybackState::Playing => "Playing",
                PlaybackState::Paused => "Paused",
                PlaybackState::Stopped => "Stopped",
            };
            
            println!("  {} State:    {}", state_emoji, state_str);
            
            if let Some(track) = status.current_track {
                let filename = std::path::Path::new(&track)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&track);
                println!("  ? Track:    {}", filename);
            } else {
                println!("  ? Track:    None");
            }
            
            if status.duration > 0.0 {
                println!("  ? Time:     {:.0}s / {:.0}s", status.position, status.duration);
            }
            
            println!("  ?? Volume:   {}%", status.volume);
            println!("  ?? Playlist: {} tracks", status.playlist_length);
            
            if let Some(index) = status.current_index {
                println!("  # Position: {} of {}", index + 1, status.playlist_length);
            }
            
            println!();
            Ok(())
        }
        Ok(_) => {
            eprintln!("? Unexpected response");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("? Failed to get status: {}", e);
            eprintln!("  Make sure the daemon is running: musicplayer daemon start");
            std::process::exit(1);
        }
    }
}

pub async fn show_playlist() -> Result<()> {
    match IpcClient::send_command(Command::GetPlaylist).await {
        Ok(Response::Playlist(tracks)) => {
            if tracks.is_empty() {
                println!("Playlist is empty");
                println!("Add tracks with: musicplayer add <path>");
                return Ok(());
            }
            
            println!("??????????????????????????????????????????");
            println!("?            Current Playlist            ?");
            println!("??????????????????????????????????????????");
            println!();
            
            for (i, track) in tracks.iter().enumerate() {
                let filename = std::path::Path::new(track)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(track);
                println!("  {}. {}", i + 1, filename);
            }
            
            println!();
            println!("Total: {} tracks", tracks.len());
            Ok(())
        }
        Ok(_) => {
            eprintln!("? Unexpected response");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("? Failed to get playlist: {}", e);
            eprintln!("  Make sure the daemon is running: musicplayer daemon start");
            std::process::exit(1);
        }
    }
}

pub async fn stop_daemon() -> Result<()> {
    let pid_file = get_pid_file()?;
    
    if !pid_file.exists() {
        println!("Daemon is not running");
        return Ok(());
    }
    
    // Try graceful shutdown first
    match IpcClient::send_command(Command::Shutdown).await {
        Ok(_) => {
            println!("? Daemon stopped");
            let _ = fs::remove_file(&pid_file);
            Ok(())
        }
        Err(_) => {
            // If IPC fails, try to kill the process
            let pid_str = fs::read_to_string(&pid_file)?;
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                kill_process(pid)?;
                println!("? Daemon stopped (forced)");
                let _ = fs::remove_file(&pid_file);
                Ok(())
            } else {
                eprintln!("? Failed to parse PID file");
                std::process::exit(1);
            }
        }
    }
}

pub async fn daemon_status() -> Result<()> {
    let pid_file = get_pid_file()?;
    
    if !pid_file.exists() {
        println!("Daemon is not running");
        return Ok(());
    }
    
    let pid_str = fs::read_to_string(&pid_file)?;
    if let Ok(pid) = pid_str.trim().parse::<i32>() {
        if is_process_running(pid) {
            println!("Daemon is running (PID: {})", pid);
            
            // Try to get player status
            if let Ok(Response::Status(status)) = IpcClient::send_command(Command::GetStatus).await {
                println!("  State: {:?}", status.state);
                println!("  Playlist: {} tracks", status.playlist_length);
            }
        } else {
            println!("Daemon is not running (stale PID file)");
            let _ = fs::remove_file(&pid_file);
        }
    } else {
        println!("Daemon status unknown (invalid PID file)");
    }
    
    Ok(())
}

fn get_pid_file() -> Result<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "musicplayer")
        .context("Failed to get project directories")?;
    
    let data_dir = dirs.data_dir();
    fs::create_dir_all(data_dir)?;
    
    Ok(data_dir.join("daemon.pid"))
}

fn is_process_running(pid: i32) -> bool {
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

fn kill_process(pid: i32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output()
            .context("Failed to kill process")?;
        Ok(())
    }
    
    #[cfg(not(unix))]
    {
        Err(anyhow::anyhow!("Process killing not implemented for this platform"))
    }
}
