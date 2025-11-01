use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation, Scale, ProgressBar, ScrolledWindow, ListBox, ListBoxRow};
use std::sync::Arc;
use tokio::sync::Mutex;
use tray_icon::{TrayIconBuilder, Icon, menu::Menu};
use std::path::Path;

use crate::ipc::{Command, IpcClient, PlaybackState, Response};

#[derive(Clone)]
struct PlayerStatus {
    state: PlaybackState,
    current_track: Option<String>,
    position: f64,
    duration: f64,
    volume: u8,
    playlist_length: usize,
    current_index: Option<usize>,
    playlist: Vec<String>,
}

pub fn start_gui_with_daemon() {
    eprintln!("🎵 Music Player GUI Launcher");
    eprintln!("============================");

    // Check if we have a display
    let has_display = std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();

    if !has_display {
        eprintln!("❌ No display found! This GUI app needs a graphical desktop environment.");
        eprintln!("");
        eprintln!("Solutions:");
        eprintln!("  1. Run this directly on your local Sway/Wayland machine (not SSH)");
        eprintln!("  2. If using SSH, enable X11 forwarding: ssh -X username@host");
        eprintln!("  3. For Wayland/Sway, ensure WAYLAND_DISPLAY is set");
        eprintln!("");
        eprintln!("Alternatively, use the TUI version: cargo run -- tui");
        std::process::exit(1);
    }

    // Environment setup for Sway/Wayland
    eprintln!("🔧 Display environment:");
    eprintln!("   WAYLAND_DISPLAY: {:?}", std::env::var("WAYLAND_DISPLAY"));
    eprintln!("   DISPLAY: {:?}", std::env::var("DISPLAY"));
    eprintln!("   XDG_SESSION_TYPE: {:?}", std::env::var("XDG_SESSION_TYPE"));

    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        eprintln!("✅ Detected Wayland environment (Sway)");
        // On Sway, try X11 first since GTK4 Wayland support might be incomplete
        std::env::set_var("GDK_BACKEND", "x11");
        eprintln!("🔧 Using X11 backend (XWayland) for GTK4 compatibility");
    } else if std::env::var("DISPLAY").is_ok() {
        eprintln!("✅ Detected X11 environment");
        std::env::set_var("GDK_BACKEND", "x11");
    } else {
        eprintln!("⚠️  No display detected, trying auto-detection");
        std::env::set_var("GDK_BACKEND", "x11");
    }

    // GTK settings for better compatibility
    std::env::set_var("GTK_CSD", "0"); // Disable client-side decorations

    eprintln!("🚀 Starting daemon and GUI...");

    eprintln!("🔧 Starting daemon...");

    // Start daemon in background thread
    eprintln!("🔧 Checking daemon status...");
    {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let daemon_running = rt.block_on(async {
            IpcClient::send_command(Command::GetStatus).await.is_ok()
        });

        if !daemon_running {
            eprintln!("🔄 Starting music daemon in background thread...");
            // Spawn daemon in background thread
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Err(e) = crate::daemon::start().await {
                        eprintln!("❌ Daemon thread failed: {}", e);
                    }
                });
            });

            // Wait a bit for daemon to start
            std::thread::sleep(std::time::Duration::from_secs(2));

            // Verify daemon is running
            let daemon_ready = rt.block_on(async {
                IpcClient::send_command(Command::GetStatus).await.is_ok()
            });

            if daemon_ready {
                eprintln!("✅ Daemon started successfully");
            } else {
                eprintln!("❌ Daemon failed to start properly");
                std::process::exit(1);
            }
        } else {
            eprintln!("✅ Daemon already running");
        }
    }

    eprintln!("🎨 Launching GTK GUI...");

    // Try to initialize GTK with better error handling

    // First try with current settings (should be X11 on Wayland)
    if let Ok(()) = gtk4::init() {
        eprintln!("✅ GTK initialized successfully");
    } else {
        eprintln!("❌ Failed to initialize GTK with X11 backend, trying Wayland...");

        // Try with Wayland backend as fallback
        std::env::set_var("GDK_BACKEND", "wayland");
        if let Ok(()) = gtk4::init() {
            eprintln!("✅ GTK initialized successfully with Wayland backend");
        } else {
            eprintln!("❌ All GTK backends failed!");
            eprintln!("");
            eprintln!("This means GTK cannot access your display.");
            eprintln!("");
            eprintln!("Possible solutions:");
            eprintln!("  1. Make sure XWayland is installed: pacman -S xorg-xwayland");
            eprintln!("  2. Make sure you're on a graphical desktop (Sway/Wayland)");
            eprintln!("  3. Check that WAYLAND_DISPLAY or DISPLAY is set");
            eprintln!("  4. Try running: export DISPLAY=:0");
            eprintln!("  5. Or use the TUI: cargo run -- tui");
            std::process::exit(1);
        }
    }

    // Create GTK application
    let app_result = Application::builder()
        .application_id("com.example.musicplayer")
        .build();

    app_result.connect_activate(move |app| {
        eprintln!("🎛️ Building music player interface...");
        build_ui(app);
    });

    eprintln!("🎵 Music Player is starting up...");
    eprintln!("   (If no window appears, check your display settings)");
    eprintln!("");

    // Run GTK main loop
    let exit_code = app_result.run();
    eprintln!("GTK application exited with code: {:?}", exit_code);

    // If GTK failed to initialize properly, show helpful message
    if exit_code != gtk4::glib::ExitCode::SUCCESS {
        eprintln!("");
        eprintln!("❌ GTK GUI failed to start (exit code: {:?})", exit_code);
        eprintln!("");
        eprintln!("This usually means:");
        eprintln!("  • No graphical display available");
        eprintln!("  • X11/Wayland forwarding not working in SSH");
        eprintln!("  • GTK/display server compatibility issues");
        eprintln!("");
        eprintln!("Try running on your local desktop:");
        eprintln!("  cargo run");
        eprintln!("");
        eprintln!("Or use the terminal interface:");
        eprintln!("  cargo run -- tui");
    }
}



fn build_ui(app: &Application) {
    eprintln!("Building UI...");

    eprintln!("Creating window...");
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Music Player")
        .default_width(600)
        .default_height(400)
        .build();

    window.present();
    window.show();
}

fn setup_system_tray(window: &ApplicationWindow) {
    // Create a simple icon (you might want to load a proper icon file)
    // For now, create a simple brown icon
    let mut icon_data = vec![0; 32 * 32 * 4];
    // Fill with brown color
    for i in 0..(32 * 32) {
        icon_data[i * 4] = 0x4A;     // R
        icon_data[i * 4 + 1] = 0x2C; // G
        icon_data[i * 4 + 2] = 0x1A; // B
        icon_data[i * 4 + 3] = 255;  // A
    }
    let icon = Icon::from_rgba(icon_data, 32, 32).unwrap();

    let menu = Menu::new();
    // Note: tray-icon menu items would go here, but keeping it simple for now

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Music Player")
        .build()
        .unwrap();

    // Connect minimize to tray behavior
    window.connect_close_request(|window| {
        window.set_visible(false);
        // Window is hidden, icon remains in tray
        gtk4::glib::Propagation::Stop
    });
}