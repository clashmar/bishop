use bishop::Color;
use widgets::theme::Theme;

use super::ThemePreset;

fn bishop_theme() -> Theme {
    Theme {
        primary: Color::new(0.086, 0.420, 0.486, 1.0),
        secondary: Color::new(0.051, 0.239, 0.278, 1.0),
        background: Color::new(0.043, 0.118, 0.137, 1.0),
        surface: Color::new(0.071, 0.176, 0.208, 1.0),
        text: Color::new(0.878, 0.910, 0.918, 1.0),
        text_muted: Color::new(0.478, 0.604, 0.639, 1.0),
        accent: Color::new(0.086, 0.420, 0.486, 1.0),
        border: Color::new(0.102, 0.227, 0.259, 1.0),
        hover: Color::new(0.114, 0.310, 0.361, 1.0),
        danger: Color::new(0.753, 0.224, 0.169, 1.0),
        selection: Color::new(0.086, 0.420, 0.486, 0.25),
        highlight: Color::new(0.086, 0.420, 0.486, 1.0),
        placeholder: Color::new(0.086, 0.420, 0.486, 0.22),
        card: Color::new(0.071, 0.176, 0.208, 1.0),
        grid: Color::new(0.086, 0.420, 0.486, 0.15),
        overlay: Color::new(0.0, 0.0, 0.0, 0.6),
        panel: Color::new(0.086, 0.420, 0.486, 1.0),
        panel_text: Color::new(1.0, 1.0, 1.0, 1.0),
    }
}

inventory::submit! {
    ThemePreset {
        name: "Bishop",
        build: bishop_theme,
    }
}


