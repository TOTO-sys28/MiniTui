# 🎵 Music Player

A minimal, cross-platform music player with TUI interface, daemon architecture, and accurate progress tracking.

## ✨ Features

- 🌍 **Cross-platform**: Works on Linux, Windows, and macOS
- 🖥️ **TUI Interface**: Terminal-based user interface with keyboard controls
- ⚙️ **Daemon Architecture**: Background process for reliable playback
- 📊 **Accurate Progress Bar**: Real-time position tracking during playback
- 🎧 **Multiple Audio Formats**: MP3, FLAC, WAV, OGG, Opus, M4A, AAC, WMA, APE, AIFF
- 📋 **Playlist Management**: Add tracks, navigate playlist
- 🔊 **Volume Control**: Adjust playback volume
- 🚀 **Auto-daemon Start**: TUI automatically starts daemon if not running

## 🚀 Installation

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs)
- **Linux**: ALSA development libraries (`alsa-lib` on most distros)
- **Windows**: No additional dependencies required
- **macOS**: No additional dependencies required

### Build from source

```bash
# Clone repository
git clone [<repository-url>](https://github.com/TOTO-sys28/MiniTui.git)
cd musicplayer

# Build
cargo build --release

# Install globally (optional)
cargo install --path .
```

### Pre-built binaries

Pre-built binaries are available in the `releases/` directory:

- **Linux**: `musicplayer-linux-x86_64` (full-featured with TUI interface)
- **Windows**: `minitui-windows.zip` (complete package with installer and TUI interface)
- **macOS**: Coming soon

#### Windows Installation

1. Download `minitui-windows.zip`
2. Extract the contents
3. Run `setup_windows.bat` as Administrator
4. Open Command Prompt or PowerShell and type `minitui`

The Windows version now includes the full TUI interface and works just like the Linux version!

For Linux, you can also download `musicplayer-linux-x86_64.tar.gz` for easy distribution.

### Building releases

To build for multiple platforms (requires cross-compilation setup):

```bash
./release.sh
```

This creates optimized binaries in the `releases/` directory.

## 🎮 Usage

### TUI Mode (Recommended)

```bash
# Auto-starts daemon if needed
cargo run -- tui

# Or if installed globally
musicplayer  # or minitui on Windows
```

**Note**: On Windows, after installation, simply type `minitui` in Command Prompt or PowerShell.

### Manual Daemon Control

```bash
# Start daemon
cargo run -- daemon start

# Use TUI in another terminal
cargo run -- tui

# Legacy CLI commands
cargo run -- play "song.mp3"
cargo run -- pause
cargo run -- next
```

### Controls

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate playlist |
| `Enter` | Play selected track |
| `Space` | Play/Pause |
| `n` | Next track |
| `p` | Previous track |
| `+/=` | Volume up |
| `-` | Volume down |
| `a` | Add files |
| `q` | Quit |

### CLI Commands

```bash
# Add music to playlist
musicplayer add ~/Music          # Linux/macOS
minitui add "C:\Music"           # Windows

musicplayer add ~/Music/song.mp3
minitui add "C:\Music\song.mp3"   # Windows

# Playback controls
musicplayer play           # Start/resume playback
musicplayer pause          # Pause
musicplayer stop           # Stop
musicplayer next           # Next track
musicplayer prev           # Previous track

# Volume control
musicplayer volume 75      # Set volume to 75%

# View status
musicplayer status         # Show current status
musicplayer playlist       # Show playlist
```

## 🎵 Example Workflow

```bash
# Start TUI (auto-starts daemon)
musicplayer

# Or manually:
# Start daemon
musicplayer daemon start

# Add your music collection
musicplayer add ~/Music

# Start playing
musicplayer play

# Check what's playing
musicplayer status

# Control from anywhere
musicplayer next
musicplayer volume 80
```

## 🏗️ Project Structure

```
musicplayer/
├── src/
│   ├── main.rs         # CLI entry point, command parsing, dispatches to TUI/daemon/GUI
│   ├── daemon.rs       # Background audio daemon, handles playback logic
│   ├── player.rs       # Audio player implementation using Rodio, MP3 decoding
│   ├── playlist.rs     # Playlist management and track navigation
│   ├── ipc.rs          # TCP-based inter-process communication
│   ├── tui.rs          # Terminal user interface using Ratatui
│   ├── cli.rs          # Legacy CLI commands for daemon control
│   ├── gui.rs          # GTK4 GUI interface (currently minimal)
│   └── theme.rs        # TUI color themes and styling
├── Cargo.toml          # Dependencies and build configuration
├── Cargo.lock          # Dependency lock file
└── README.md           # This file
```

## 🛠️ Development

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
```

## 🧪 Technical Details

### Architecture

The application uses a client-server architecture:

1. **Daemon Process**: Background audio player using Rodio library
2. **TUI Client**: Terminal interface that communicates with daemon via TCP
3. **IPC Layer**: JSON-based communication over TCP sockets (127.0.0.1:12345)

### Audio Playback

- Uses Rodio for cross-platform audio output
- Special MP3 handling with minimp3 decoder for better compatibility
- Accurate position tracking using system timers
- Background daemon ensures uninterrupted playback

### Cross-Platform Support

**Linux**: Uses ALSA/PulseAudio through Rodio
**Windows**: Uses WASAPI/DirectSound through Rodio
**macOS**: Uses CoreAudio through Rodio
**Communication**: TCP sockets work identically across platforms

## 🐛 Troubleshooting

### Common Issues

**"No such device or address"**: Run in proper terminal (not SSH)
**"Failed to decode audio file"**: Check file format support
**Port already in use**: Kill existing daemon process

### Daemon Management

```bash
# Check if running
ps aux | grep musicplayer

# Kill daemon
pkill musicplayer

# Clean socket (Linux only)
rm /tmp/musicplayer.sock
```

## ?? License

MIT License - See LICENSE file for details

## ?? Contributing

Contributions welcome! Please feel free to submit a Pull Request.

## 📜 Project History

This project evolved through several iterations:

1. **Initial**: Basic CLI player
2. **TUI Addition**: Terminal interface with Ratatui
3. **Daemon Architecture**: Background process for reliability
4. **Progress Tracking**: Accurate position using timers
5. **MP3 Fixes**: Custom MP3 decoder for compatibility
6. **Cross-Platform**: TCP IPC replacing Unix sockets
7. **Auto-start**: TUI automatically manages daemon

### Key Improvements Made

- **Progress Bar**: Implemented real-time position tracking using `Instant` timers instead of relying on Rodio's limited position API
- **MP3 Support**: Added minimp3 decoder for better compatibility with problematic MP3 files
- **Cross-Platform IPC**: Converted Unix socket communication to TCP sockets for Windows/Linux compatibility
- **Auto-Daemon**: Modified TUI to automatically start daemon if not running
- **Code Cleanup**: Removed unused code, fixed compilation warnings, optimized build size

## 🎯 Current Status

The project is now a **fully functional MVP** music player with accurate progress tracking and reliable MP3 playback. The TUI provides an excellent user experience, and the daemon architecture ensures smooth operation across different systems.

### Future Enhancements (Optional)

- **Visualizer**: Add audio visualization in TUI
- **Better Playlist Management**: Shuffle, repeat modes, queue management
- **Additional Formats**: Support for more audio codecs
- **MPRIS Integration**: Media player remote interface for Linux desktop integration
- **Configuration**: Custom themes, keybindings, and settings

## 🙏 Credits

Built with:

- **Rodio**: Cross-platform audio library
- **minimp3**: Pure Rust MP3 decoder for compatibility
- **Symphonia**: Fallback decoder for other formats
- **Tokio**: Async runtime
- **Ratatui**: Terminal user interface framework
- **Clap**: CLI parser

Made with ❤️ for terminal music lovers
