use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug)]
pub enum Theme {
    Default,
    Dark,
    Light,
    Green,
    Blue,
    Purple,
    Cyan,
    Red,
    Coffee,
}

impl Theme {
    pub fn all() -> Vec<Theme> {
        vec![
            Theme::Default,
            Theme::Dark,
            Theme::Light,
            Theme::Green,
            Theme::Blue,
            Theme::Purple,
            Theme::Cyan,
            Theme::Red,
            Theme::Coffee,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Theme::Default => "Default",
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Green => "Green",
            Theme::Blue => "Blue",
            Theme::Purple => "Purple",
            Theme::Cyan => "Cyan",
            Theme::Red => "Red",
            Theme::Coffee => "Coffee",
        }
    }

    pub fn status_color(&self) -> Color {
        match self {
            Theme::Default => Color::Green,
            Theme::Dark => Color::White,
            Theme::Light => Color::Black,
            Theme::Green => Color::Green,
            Theme::Blue => Color::Blue,
            Theme::Purple => Color::Magenta,
            Theme::Cyan => Color::Cyan,
            Theme::Red => Color::Red,
            Theme::Coffee => Color::Yellow,
        }
    }

    pub fn now_playing_color(&self) -> Color {
        match self {
            Theme::Default => Color::Cyan,
            Theme::Dark => Color::Cyan,
            Theme::Light => Color::Blue,
            Theme::Green => Color::Green,
            Theme::Blue => Color::Blue,
            Theme::Purple => Color::Magenta,
            Theme::Cyan => Color::Cyan,
            Theme::Red => Color::Red,
            Theme::Coffee => Color::White,
        }
    }

    pub fn playlist_color(&self) -> Color {
        match self {
            Theme::Default => Color::Yellow,
            Theme::Dark => Color::Yellow,
            Theme::Light => Color::DarkGray,
            Theme::Green => Color::Green,
            Theme::Blue => Color::Blue,
            Theme::Purple => Color::Magenta,
            Theme::Cyan => Color::Cyan,
            Theme::Red => Color::Red,
            Theme::Coffee => Color::Yellow,
        }
    }

    pub fn controls_color(&self) -> Color {
        match self {
            Theme::Default => Color::Magenta,
            Theme::Dark => Color::Cyan,
            Theme::Light => Color::DarkGray,
            Theme::Green => Color::Green,
            Theme::Blue => Color::Blue,
            Theme::Purple => Color::Magenta,
            Theme::Cyan => Color::Cyan,
            Theme::Red => Color::Red,
            Theme::Coffee => Color::White,
        }
    }

    pub fn file_browser_color(&self) -> Color {
        match self {
            Theme::Default => Color::Cyan,
            Theme::Dark => Color::Cyan,
            Theme::Light => Color::Blue,
            Theme::Green => Color::Green,
            Theme::Blue => Color::Blue,
            Theme::Purple => Color::Magenta,
            Theme::Cyan => Color::Cyan,
            Theme::Red => Color::Red,
            Theme::Coffee => Color::White,
        }
    }

    pub fn highlight_bg(&self) -> Color {
        match self {
            Theme::Default => Color::Blue,
            Theme::Dark => Color::Gray,
            Theme::Light => Color::Gray,
            Theme::Green => Color::Green,
            Theme::Blue => Color::Blue,
            Theme::Purple => Color::Magenta,
            Theme::Cyan => Color::Cyan,
            Theme::Red => Color::Red,
            Theme::Coffee => Color::Gray,
        }
    }

    pub fn highlight_fg(&self) -> Color {
        match self {
            Theme::Default => Color::White,
            Theme::Dark => Color::White,
            Theme::Light => Color::White,
            Theme::Green => Color::White,
            Theme::Blue => Color::White,
            Theme::Purple => Color::White,
            Theme::Cyan => Color::White,
            Theme::Red => Color::White,
            Theme::Coffee => Color::Black,
        }
    }

    pub fn gauge_color(&self) -> Color {
        self.now_playing_color()
    }
}

pub struct ThemeStyle {
    pub theme: Theme,
}

impl ThemeStyle {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    pub fn status_style(&self) -> Style {
        Style::default().fg(self.theme.status_color())
    }

    pub fn now_playing_style(&self) -> Style {
        Style::default().fg(self.theme.now_playing_color())
    }

    pub fn playlist_style(&self) -> Style {
        Style::default().fg(self.theme.playlist_color())
    }

    pub fn controls_style(&self) -> Style {
        Style::default().fg(self.theme.controls_color())
    }

    pub fn file_browser_style(&self) -> Style {
        Style::default().fg(self.theme.file_browser_color())
    }

    pub fn highlight_style(&self) -> Style {
        Style::default()
            .bg(self.theme.highlight_bg())
            .fg(self.theme.highlight_fg())
            .add_modifier(Modifier::BOLD)
    }

    pub fn gauge_style(&self) -> Style {
        Style::default().fg(self.theme.gauge_color())
    }
}
