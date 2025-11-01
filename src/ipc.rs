use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const SOCKET_ADDR: &str = "127.0.0.1:12345";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Play { path: Option<String> },
    Pause,
    Stop,
    Next,
    Previous,
    SetVolume { level: u8 },
    AddTracks { paths: Vec<String> },
    GetStatus,
    GetPlaylist,
    ClearPlaylist,
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Response {
    Ok,
    Status(PlayerStatus),
    Playlist(Vec<String>),
    Error(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerStatus {
    pub state: PlaybackState,
    pub current_track: Option<String>,
    pub position: f64,
    pub duration: f64,
    pub volume: u8,
    pub playlist_length: usize,
    pub current_index: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

pub fn get_socket_addr() -> &'static str {
    SOCKET_ADDR
}

pub struct IpcServer {
    listener: TcpListener,
}

impl IpcServer {
    pub async fn new() -> Result<Self> {
        let addr = get_socket_addr();

        let listener = TcpListener::bind(addr)
            .await
            .context("Failed to bind IPC socket")?;

        Ok(Self { listener })
    }

    pub async fn accept(&self) -> Result<IpcConnection> {
        let (stream, _) = self.listener.accept().await
            .context("Failed to accept connection")?;
        
        Ok(IpcConnection { stream })
    }
}

pub struct IpcConnection {
    stream: TcpStream,
}

impl IpcConnection {
    pub async fn recv(&mut self) -> Result<Command> {
        let mut reader = BufReader::new(&mut self.stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        
        let command: Command = serde_json::from_str(&line)
            .context("Failed to parse command")?;
        
        Ok(command)
    }

    pub async fn send(&mut self, response: Response) -> Result<()> {
        let json = serde_json::to_string(&response)?;
        self.stream.write_all(format!("{}\n", json).as_bytes()).await?;
        self.stream.flush().await?;
        Ok(())
    }
}

pub struct IpcClient;

impl IpcClient {
    pub async fn send_command(command: Command) -> Result<Response> {
        let addr = get_socket_addr();

        let mut stream = TcpStream::connect(addr).await
            .context("Failed to connect to socket")?;

        // Send command
        let json = serde_json::to_string(&command)?;
        stream.write_all(format!("{}\n", json).as_bytes()).await?;
        stream.flush().await?;

        // Read response
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: Response = serde_json::from_str(&line)
            .context("Failed to parse response")?;

        Ok(response)
    }
}
