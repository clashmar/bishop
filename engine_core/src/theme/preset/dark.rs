use bishop::Color;
use widgets::theme::Theme;

use super::ThemePreset;

fn dark_theme() -> Theme {
    Theme {
        background: Color::new(0.05, 0.05, 0.08, 1.0),
        surface: Color::new(0.10, 0.10, 0.14, 1.0),
        text: Color::new(0.90, 0.90, 0.95, 1.0),
        text_muted: Color::new(0.55, 0.55, 0.60, 1.0),
        border: Color::new(0.25, 0.25, 0.30, 1.0),
        hover: Color::new(0.20, 0.20, 0.28, 1.0),
        selection: Color::new(0.706, 0.824, 1.0, 0.25),
        highlight: Color::YELLOW,
        placeholder: Color::new(0.2, 0.85, 0.35, 0.22),
        card: Color::new(0.20, 0.20, 0.24, 1.0),
        overlay: Color::BLACK,
        panel: Color::new(0.28, 0.28, 0.32, 1.0),
        panel_text: Color::WHITE,
        ..Theme::default()
    }
}

inventory::submit! {
    ThemePreset {
        name: "Dark",
        build: dark_theme,
    }
}
