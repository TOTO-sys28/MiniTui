use anyhow::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph};
use std::fs;
use std::path::{Path, PathBuf};
use std::io;
use std::time::{Duration, Instant};
use tokio::time::{interval, Duration as TokioDuration};

use crate::theme::{Theme, ThemeStyle};

use crate::ipc::{Command, IpcClient, PlaybackState, Response};

pub struct Tui {
    terminal: Terminal<CrosstermBackend<std::io::Stderr>>,
    theme: ThemeStyle,
}

#[derive(Clone)]
pub struct PlayerStatus {
    pub state: PlaybackState,
    pub current_track: Option<String>,
    pub position: f64,
    pub duration: f64,
    pub volume: u8,
    pub playlist_length: usize,
    pub current_index: Option<usize>,
    pub playlist: Vec<String>,
}

enum AppMode {
    Player,
    FileBrowser,
}

#[derive(Clone)]
struct FileEntry {
    path: PathBuf,
    is_dir: bool,
    is_audio: bool,
}

pub struct FileBrowser {
    current_path: PathBuf,
    entries: Vec<FileEntry>,
    selected: usize,
    scroll_offset: usize,
}

impl FileBrowser {
    fn new() -> Result<Self> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let path = PathBuf::from(home);
        let mut browser = Self {
            current_path: path.clone(),
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
        };
        browser.refresh()?;
        Ok(browser)
    }

    fn refresh(&mut self) -> Result<()> {
        self.entries.clear();
        
        // Add parent directory entry if not at root
        if let Some(parent) = self.current_path.parent() {
            self.entries.push(FileEntry {
                path: parent.to_path_buf(),
                is_dir: true,
                is_audio: false,
            });
        }
        
        // Read directory entries
        match fs::read_dir(&self.current_path) {
            Ok(entries) => {
                let mut dirs: Vec<FileEntry> = Vec::new();
                let mut audio_files: Vec<FileEntry> = Vec::new();
                
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        // Ensure we have absolute path - canonicalize if possible, otherwise use as-is
                        let abs_path = if path.is_absolute() {
                            path.clone()
                        } else {
                            self.current_path.join(&path)
                        };
                        
                        // Try to canonicalize, but don't fail if it doesn't work (e.g., broken symlinks)
                        let abs_path = abs_path.canonicalize().unwrap_or_else(|_| {
                            // If canonicalize fails, ensure it's at least absolute
                            if abs_path.is_absolute() {
                                abs_path.clone()
                            } else {
                                std::env::current_dir()
                                    .unwrap_or_else(|_| PathBuf::from("/"))
                                    .join(&abs_path)
                            }
                        });
                        
                        let is_dir = abs_path.is_dir();
                        
                        let file_entry = FileEntry {
                            path: abs_path.clone(),
                            is_dir,
                            is_audio: !is_dir && is_audio_file(&abs_path),
                        };
                        
                        if is_dir {
                            dirs.push(file_entry);
                        } else if file_entry.is_audio {
                            audio_files.push(file_entry);
                        }
                    }
                }
                
                // Sort
                dirs.sort_by(|a, b| a.path.cmp(&b.path));
                audio_files.sort_by(|a, b| a.path.cmp(&b.path));
                
                // Add directories first, then audio files
                self.entries.extend(dirs);
                self.entries.extend(audio_files);
            }
            Err(_) => {
                // If we can't read, go back to parent
                if let Some(parent) = self.current_path.parent() {
                    self.current_path = parent.to_path_buf();
                    return self.refresh();
                }
            }
        }
        
        // Keep selected within bounds
        if !self.entries.is_empty() {
            if self.selected >= self.entries.len() {
                self.selected = self.entries.len().saturating_sub(1);
            }
        } else {
            self.selected = 0;
        }
        
        // Update scroll offset
        self.update_scroll();
        
        Ok(())
    }

    fn update_scroll(&mut self) {
        let visible_height = 20; // Assume ~20 visible items
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected.saturating_sub(visible_height - 1);
        }
    }

    fn navigate_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.update_scroll();
        }
    }

    fn navigate_down(&mut self) {
        if self.selected < self.entries.len().saturating_sub(1) {
            self.selected += 1;
            self.update_scroll();
        }
    }

    fn enter_directory(&mut self) -> Result<()> {
        if self.entries.is_empty() || self.selected >= self.entries.len() {
            return Ok(());
        }
        
        let entry = &self.entries[self.selected];
        
        // Check if it's parent directory (first entry and is parent)
        if self.selected == 0 {
            if let Some(parent) = self.current_path.parent() {
                if entry.path == *parent {
                    // It's the parent directory entry
                    self.current_path = entry.path.clone();
                    self.selected = 0;
                    self.scroll_offset = 0;
                    self.refresh()?;
                    return Ok(());
                }
            }
        }
        
        // Regular directory navigation
        if entry.is_dir {
            self.current_path = entry.path.clone();
            self.selected = 0;
            self.scroll_offset = 0;
            self.refresh()?;
        }
        
        Ok(())
    }

    fn go_to_parent(&mut self) -> Result<()> {
        if let Some(parent) = self.current_path.parent() {
            self.current_path = parent.to_path_buf();
            self.selected = 0;
            self.scroll_offset = 0;
            self.refresh()?;
        }
        Ok(())
    }

    fn get_selected_path(&self) -> Option<PathBuf> {
        if self.entries.is_empty() || self.selected >= self.entries.len() {
            return None;
        }
        
        let entry = &self.entries[self.selected];
        
        // For parent directory, return None
        if self.selected == 0 {
            if let Some(parent) = self.current_path.parent() {
                if entry.path == *parent {
                    return None;
                }
            }
        }
        
        // Make path absolute if it's relative
        if entry.path.is_absolute() {
            Some(entry.path.clone())
        } else {
            Some(self.current_path.join(&entry.path))
        }
    }
}

fn is_audio_file(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    
    matches!(ext.as_deref(), 
        Some("mp3") | Some("flac") | Some("wav") | Some("ogg") | 
        Some("opus") | Some("m4a") | Some("aac") | Some("wma") | 
        Some("ape") | Some("aiff"))
}

impl Tui {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stderr = io::stderr();
        execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stderr);
        let terminal = Terminal::new(backend)?;

        Ok(Self { 
            terminal,
            theme: ThemeStyle::new(Theme::Default),
        })
    }
    
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = ThemeStyle::new(theme);
    }

    pub async fn run(mut self) -> Result<()> {
        let mut status = PlayerStatus {
            state: PlaybackState::Stopped,
            current_track: None,
            position: 0.0,
            duration: 0.0,
            volume: 70,
            playlist_length: 0,
            current_index: None,
            playlist: Vec::new(),
        };

        let mut mode = AppMode::Player;
        let mut file_browser = match FileBrowser::new() {
            Ok(browser) => browser,
            Err(e) => {
                self.restore()?;
                return Err(anyhow::anyhow!("Failed to initialize file browser: {}", e));
            }
        };
        
        // Command throttling to prevent rapid-fire commands
        let mut last_volume_change = Instant::now();
        let mut last_command = Instant::now();
        let volume_debounce = Duration::from_millis(150);
        let command_debounce = Duration::from_millis(50);
        
        let mut status_tick = interval(TokioDuration::from_millis(800));
        let mut last_status_update = Instant::now();

        loop {
            // Use tick-based status updates instead of elapsed time to be more consistent
            tokio::select! {
                _ = status_tick.tick() => {
                    // Fetch status from daemon
                    if let Ok(Response::Status(s)) = IpcClient::send_command(Command::GetStatus).await {
                        status.state = s.state;
                        status.current_track = s.current_track;
                        status.position = s.position;
                        status.duration = s.duration;
                        status.volume = s.volume;
                        status.playlist_length = s.playlist_length;
                        status.current_index = s.current_index;
                    }

                    // Fetch playlist less frequently (every 3rd tick)
                    if last_status_update.elapsed().as_millis() > 2400 {
                        last_status_update = Instant::now();
                        if let Ok(Response::Playlist(p)) = IpcClient::send_command(Command::GetPlaylist).await {
                            status.playlist = p;
                        }
                    }
                }
                _ = tokio::time::sleep(TokioDuration::from_millis(50)) => {
                    // Continue to input handling
                }
            }
            
            // Handle keyboard input
            let has_input = crossterm::event::poll(Duration::from_millis(10)).unwrap_or(false);
            
            if has_input {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Press {
                        // Check for Ctrl+D (exit)
                        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('d') {
                            break;
                        }
                        
                        match &mut mode {
                            AppMode::Player => {
                                match key.code {
                                    KeyCode::Char('q') | KeyCode::Esc => break,
                                    KeyCode::Char('f') => {
                                        mode = AppMode::FileBrowser;
                                        file_browser = FileBrowser::new()?;
                                    }
                                    KeyCode::Char(' ') => {
                                        // Debounce play/pause commands
                                        if last_command.elapsed() >= command_debounce {
                                            let cmd = if status.state == PlaybackState::Playing {
                                                Command::Pause
                                            } else {
                                                Command::Play { path: None }
                                            };
                                            let _ = IpcClient::send_command(cmd).await;
                                            last_command = Instant::now();
                                        }
                                    }
                                    KeyCode::Char('p') => {
                                        // Debounce pause commands
                                        if last_command.elapsed() >= command_debounce {
                                            let _ = IpcClient::send_command(Command::Pause).await;
                                            last_command = Instant::now();
                                        }
                                    }
                                    KeyCode::Char('s') => {
                                        let _ = IpcClient::send_command(Command::Stop).await;
                                    }
                                    KeyCode::Char('n') | KeyCode::Right => {
                                        // Throttle commands
                                        if last_command.elapsed() >= command_debounce {
                                            let _ = IpcClient::send_command(Command::Next).await;
                                            last_command = Instant::now();
                                        }
                                    }
                                    KeyCode::Char('b') | KeyCode::Left => {
                                        // Throttle commands
                                        if last_command.elapsed() >= command_debounce {
                                            let _ = IpcClient::send_command(Command::Previous).await;
                                            last_command = Instant::now();
                                        }
                                    }
                                    KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Up => {
                                        // Debounce volume changes
                                        if last_volume_change.elapsed() >= volume_debounce {
                                            let new_vol = (status.volume + 5).min(100);
                                            let _ = IpcClient::send_command(Command::SetVolume { level: new_vol }).await;
                                            last_volume_change = Instant::now();
                                        }
                                    }
                                    KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Down => {
                                        // Debounce volume changes
                                        if last_volume_change.elapsed() >= volume_debounce {
                                            let new_vol = status.volume.saturating_sub(5);
                                            let _ = IpcClient::send_command(Command::SetVolume { level: new_vol }).await;
                                            last_volume_change = Instant::now();
                                        }
                                    }
                                    KeyCode::Char('t') => {
                                        // Cycle through themes
                                        let themes = Theme::all();
                                        let current_idx = themes.iter().position(|t| {
                                            format!("{:?}", t) == format!("{:?}", self.theme.theme)
                                        }).unwrap_or(0);
                                        let next_idx = (current_idx + 1) % themes.len();
                                        self.set_theme(themes[next_idx]);
                                    }
                                    _ => {}
                                }
                            }
                            AppMode::FileBrowser => {
                                match key.code {
                                    KeyCode::Char('q') | KeyCode::Esc => {
                                        mode = AppMode::Player;
                                    }
                                    KeyCode::Char('t') => {
                                        // Cycle through themes
                                        let themes = Theme::all();
                                        let current_idx = themes.iter().position(|t| {
                                            format!("{:?}", t) == format!("{:?}", self.theme.theme)
                                        }).unwrap_or(0);
                                        let next_idx = (current_idx + 1) % themes.len();
                                        self.set_theme(themes[next_idx]);
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        file_browser.navigate_up();
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        file_browser.navigate_down();
                                    }
                                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                                        if !file_browser.entries.is_empty() && file_browser.selected < file_browser.entries.len() {
                                            let entry = file_browser.entries[file_browser.selected].clone();
                                            if entry.is_dir {
                                                // Navigate into directory
                                                let _ = file_browser.enter_directory();
                                            } else if entry.is_audio {
                                                // Path is already absolute from refresh()
                                                let path_str = entry.path.to_string_lossy().to_string();
                                                
                                                // Add to playlist first
                                                let _ = IpcClient::send_command(Command::AddTracks { 
                                                    paths: vec![path_str.clone()] 
                                                }).await;
                                                // Then play it
                                                tokio::time::sleep(Duration::from_millis(100)).await;
                                                let _ = IpcClient::send_command(Command::Play { 
                                                    path: Some(path_str) 
                                                }).await;
                                                mode = AppMode::Player;
                                            }
                                        }
                                    }
                                    KeyCode::Left | KeyCode::Char('h') => {
                                        let _ = file_browser.go_to_parent();
                                    }
                                    KeyCode::Char('a') => {
                                        // Add selected item to playlist
                                        if let Some(selected_path) = file_browser.get_selected_path() {
                                            let path_str = selected_path.to_string_lossy().to_string();
                                            let _ = IpcClient::send_command(Command::AddTracks { 
                                                paths: vec![path_str] 
                                            }).await;
                                        } else {
                                            // Add current directory if nothing selected
                                            let path_str = file_browser.current_path.to_string_lossy().to_string();
                                            let _ = IpcClient::send_command(Command::AddTracks { 
                                                paths: vec![path_str] 
                                            }).await;
                                        }
                                    }
                                    KeyCode::Char('A') => {
                                        // Navigate to folder AND add all songs as playlist and start playing
                                        if !file_browser.entries.is_empty() && file_browser.selected < file_browser.entries.len() {
                                            let entry = file_browser.entries[file_browser.selected].clone();
                                            if entry.is_dir {
                                                // Get the folder path (already absolute)
                                                let folder_path = entry.path.clone();
                                                
                                                // Navigate into the folder first
                                                file_browser.current_path = folder_path.clone();
                                                file_browser.selected = 0;
                                                file_browser.scroll_offset = 0;
                                                let _ = file_browser.refresh();
                                                
                                                // Add all songs from that folder to playlist
                                                let path_str = folder_path.to_string_lossy().to_string();
                                                let _ = IpcClient::send_command(Command::AddTracks { 
                                                    paths: vec![path_str.clone()] 
                                                }).await;
                                                
                                                // Wait a bit for tracks to be added
                                                tokio::time::sleep(Duration::from_millis(300)).await;
                                                
                                                // Start playing
                                                let _ = IpcClient::send_command(Command::Play { 
                                                    path: None 
                                                }).await;
                                                
                                                mode = AppMode::Player;
                                            } else {
                                                // If it's a file, add current directory
                                                let path_str = file_browser.current_path.to_string_lossy().to_string();
                                                let _ = IpcClient::send_command(Command::AddTracks { 
                                                    paths: vec![path_str.clone()] 
                                                }).await;
                                                tokio::time::sleep(Duration::from_millis(300)).await;
                                                let _ = IpcClient::send_command(Command::Play { 
                                                    path: None 
                                                }).await;
                                                mode = AppMode::Player;
                                            }
                                        } else {
                                            // No selection, add current directory
                                            let path_str = file_browser.current_path.to_string_lossy().to_string();
                                            let _ = IpcClient::send_command(Command::AddTracks { 
                                                paths: vec![path_str.clone()] 
                                            }).await;
                                            tokio::time::sleep(Duration::from_millis(300)).await;
                                            let _ = IpcClient::send_command(Command::Play { 
                                                path: None 
                                            }).await;
                                            mode = AppMode::Player;
                                        }
                                    }
                                    KeyCode::Char('p') => {
                                        // Play selected file immediately
                                        if !file_browser.entries.is_empty() && file_browser.selected < file_browser.entries.len() {
                                            let entry = file_browser.entries[file_browser.selected].clone();
                                            if entry.is_audio {
                                                // Path is already absolute
                                                let path_str = entry.path.to_string_lossy().to_string();
                                                
                                                // First add to playlist
                                                let _ = IpcClient::send_command(Command::AddTracks { 
                                                    paths: vec![path_str.clone()] 
                                                }).await;
                                                tokio::time::sleep(Duration::from_millis(100)).await;
                                                // Then play it
                                                let _ = IpcClient::send_command(Command::Play { 
                                                    path: Some(path_str) 
                                                }).await;
                                                mode = AppMode::Player;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            // Render UI
            match mode {
                AppMode::Player => {
                    if let Err(e) = self.terminal.draw(|f| ui_player(f, &status, &self.theme)) {
                        eprintln!("Render error: {}", e);
                        break;
                    }
                }
                AppMode::FileBrowser => {
                    if let Err(e) = self.terminal.draw(|f| ui_file_browser(f, &status, &file_browser, &self.theme)) {
                        eprintln!("Render error: {}", e);
                        break;
                    }
                }
            }
        }

        self.restore()?;
        Ok(())
    }

    fn restore(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn ui_player(frame: &mut Frame, status: &PlayerStatus, theme: &ThemeStyle) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(size);

    let state_text = match status.state {
        PlaybackState::Playing => "? PLAYING",
        PlaybackState::Paused => "? PAUSED",
        PlaybackState::Stopped => "? STOPPED",
    };

    let status_text = format!("{} | Volume: {}% | Tracks: {} | Theme: {}", state_text, status.volume, status.playlist_length, theme.theme.name());
    frame.render_widget(
        Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status").style(theme.status_style())),
        chunks[0]
    );

    let now_playing_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(1), Constraint::Length(2)])
        .split(chunks[1]);

    let track_name = status.current_track
        .as_ref()
        .map(|t| {
            std::path::Path::new(t)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(t)
        })
        .unwrap_or("No track selected");

    let time_text = if status.duration > 0.0 {
        format!("{:.0}s / {:.0}s", status.position, status.duration)
    } else {
        "".to_string()
    };

    frame.render_widget(
        Paragraph::new(format!("{}\n{}", track_name, time_text))
            .block(Block::default().borders(Borders::ALL).title("Now Playing").style(theme.now_playing_style())),
        now_playing_chunks[0]
    );

    let progress = if status.duration > 0.0 {
        ((status.position / status.duration) * 100.0) as u16
    } else {
        0
    };

    frame.render_widget(
        Gauge::default()
            .block(Block::default().title("Progress"))
            .gauge_style(theme.gauge_style())
            .percent(progress),
        now_playing_chunks[2]
    );

    let playlist_items: Vec<ListItem> = status.playlist
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let filename = std::path::Path::new(track)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(track);
            let prefix = if status.current_index == Some(i) { "? " } else { "  " };
            ListItem::new(format!("{}{}. {}", prefix, i + 1, filename))
        })
        .collect();

    let playlist = List::new(playlist_items)
        .block(Block::default().borders(Borders::ALL).title("Playlist").style(Style::default().fg(Color::Yellow)))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("? ");

    let mut state = ListState::default();
    if let Some(idx) = status.current_index {
        state.select(Some(idx));
    }
    frame.render_stateful_widget(playlist, chunks[2], &mut state);

    let help_text = "[Space] Play/Pause | [S] Stop | [N/?] Next | [B/?] Prev | [+/-] Volume | [F] Files | [Q/Ctrl+D] Quit";
    frame.render_widget(
        Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Controls").style(Style::default().fg(Color::Magenta))),
        chunks[3]
    );
}

fn ui_file_browser(frame: &mut Frame, status: &PlayerStatus, browser: &FileBrowser, theme: &ThemeStyle) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(size);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(4)])
        .split(chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0), Constraint::Length(4)])
        .split(chunks[1]);

    let path_text = browser.current_path.to_string_lossy();
    frame.render_widget(
        Paragraph::new(path_text.as_ref())
            .block(Block::default().borders(Borders::ALL).title("Current Directory").style(theme.status_style())),
        left_chunks[0]
    );

    // Show visible entries based on scroll
    let visible_items: Vec<ListItem> = browser.entries
        .iter()
        .enumerate()
        .skip(browser.scroll_offset)
        .take(20)
        .map(|(idx, entry)| {
            let name = entry.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    // For parent directory
                    if idx == 0 && entry.path == browser.current_path.parent().unwrap_or(&browser.current_path) {
                        "..".to_string()
                    } else {
                        entry.path.to_string_lossy().to_string()
                    }
                });
            
            let icon = if name == ".." || entry.is_dir {
                "?? "
            } else if entry.is_audio {
                "?? "
            } else {
                "  "
            };
            
            let display_name = if name == ".." {
                ".. (parent)".to_string()
            } else {
                format!("{}{}", icon, name)
            };
            
            ListItem::new(display_name)
        })
        .collect();

    let file_list = List::new(visible_items)
        .block(Block::default().borders(Borders::ALL).title(format!("Files & Folders ({})", browser.entries.len())).style(Style::default().fg(Color::Cyan)))
        .highlight_style(theme.highlight_style())
        .highlight_symbol("? ");

    let mut state = ListState::default();
    let visible_selected = browser.selected.saturating_sub(browser.scroll_offset);
    state.select(Some(visible_selected));
    frame.render_stateful_widget(file_list, left_chunks[1], &mut state);

    let browser_help = "[??/jk] Navigate\n[Enter/?/l] Open | [?/h] Up\n[A] Add Dir | [a] Add Item\n[P] Play | [Q] Back";
    frame.render_widget(
        Paragraph::new(browser_help)
            .block(Block::default().borders(Borders::ALL).title("File Browser Controls").style(Style::default().fg(Color::Yellow))),
        left_chunks[2]
    );

    let state_text = match status.state {
        PlaybackState::Playing => "? PLAYING",
        PlaybackState::Paused => "? PAUSED",
        PlaybackState::Stopped => "? STOPPED",
    };
    
    let track_name = status.current_track
        .as_ref()
        .map(|t| {
            std::path::Path::new(t)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(t)
        })
        .unwrap_or("No track");

    frame.render_widget(
        Paragraph::new(format!("{}\n{}\nVolume: {}%\nTracks: {}", 
            state_text, track_name, status.volume, status.playlist_length))
            .block(Block::default().borders(Borders::ALL).title("Player Status").style(theme.controls_style())),
        right_chunks[0]
    );

    let playlist_items: Vec<ListItem> = status.playlist
        .iter()
        .enumerate()
        .take(15)
        .map(|(i, track)| {
            let filename = std::path::Path::new(track)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(track);
            let prefix = if status.current_index == Some(i) { "? " } else { "  " };
            ListItem::new(format!("{}{}. {}", prefix, i + 1, filename))
        })
        .collect();

    let playlist = List::new(playlist_items)
        .block(Block::default().borders(Borders::ALL).title(format!("Playlist ({})", status.playlist_length)).style(theme.playlist_style()));
    
    let mut state = ListState::default();
    if let Some(idx) = status.current_index {
        state.select(Some(idx.min(14)));
    }
    frame.render_stateful_widget(playlist, right_chunks[1], &mut state);

    let quick_help = "[Space] Play/Pause\n[S] Stop | [N] Next\n[B] Prev | [+/-] Vol";
    frame.render_widget(
        Paragraph::new(quick_help)
            .block(Block::default().borders(Borders::ALL).title("Quick Controls").style(theme.file_browser_style())),
        right_chunks[2]
    );
}

pub async fn run_tui() -> Result<()> {
    // Check if daemon is running first
    match IpcClient::send_command(Command::GetStatus).await {
        Ok(_) => {
            // Daemon is running, start TUI
            let tui = Tui::new().map_err(|e| anyhow::anyhow!("Failed to initialize TUI: {}", e))?;
            tui.run().await.map_err(|e| anyhow::anyhow!("TUI error: {}", e))?;
            Ok(())
        }
        Err(_) => {
            eprintln!("? Daemon is not running, starting it...");
            // Start daemon in background thread
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Err(e) = crate::daemon::start().await {
                        eprintln!("Failed to start daemon: {}", e);
                    }
                });
            });
            // Wait a bit for daemon to start
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            // Check again
            match IpcClient::send_command(Command::GetStatus).await {
                Ok(_) => {
                    let tui = Tui::new().map_err(|e| anyhow::anyhow!("Failed to initialize TUI: {}", e))?;
                    tui.run().await.map_err(|e| anyhow::anyhow!("TUI error: {}", e))?;
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to start daemon: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
