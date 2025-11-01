# MiniTUI Music Player - Windows Installation

## Quick Install

1. Download `minitui.exe` and `setup_windows.bat`
2. Run `setup_windows.bat` as Administrator
3. Open Command Prompt or PowerShell and type `minitui`

## Manual Installation

1. Copy `minitui.exe` to a folder (e.g., `C:\Program Files\MiniTUI\`)
2. Add the folder to your system PATH:
   - Right-click "This PC" → Properties → Advanced system settings
   - Click "Environment Variables"
   - Under "System variables", find "Path" and click "Edit"
   - Add your installation folder (e.g., `C:\Program Files\MiniTUI\`)
   - Click OK to save

3. Open a new Command Prompt/PowerShell window
4. Type `minitui` to start the application

## Usage

Once installed, you can use MiniTUI just like on Linux:

```cmd
# Start the TUI interface
minitui

# Or use CLI commands
minitui daemon start
minitui add "C:\Music"
minitui play
minitui status
```

## Features

- 🎵 Full music player with TUI interface
- 🎧 Supports MP3, FLAC, WAV, OGG, and more
- 📋 Playlist management
- 🔊 Volume control
- ⚡ Background daemon for uninterrupted playback

## Controls (in TUI mode)

- `↑/↓` - Navigate playlist
- `Enter` - Play selected track
- `Space` - Play/Pause
- `n` - Next track
- `p` - Previous track
- `+/=` - Volume up
- `-` - Volume down
- `a` - Add files
- `q` - Quit

Enjoy your music! 🎶