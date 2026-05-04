use widgets::theme::Theme;

use super::ThemePreset;

fn default_theme() -> Theme {
    Theme::default()
}

inventory::submit! {
    ThemePreset {
        name: "Default",
        build: default_theme,
    }
}
