use bishop::Color;
use widgets::theme::Theme;

use super::ThemePreset;

fn bishop_theme() -> Theme {
    let primary = Color::from_hex("22b1d0");
    let secondary = Color::from_hex("222034");
    let background = Color::from_hex("000000");
    let accent = Color::from_hex("d95763");
    let highlight = Color::from_hex("e8fbff");
    let panel = Color::from_hex("7c9ea6");

    Theme {
        primary,
        secondary,
        background,
        surface: primary,
        text: Color::from_hex("E0E8EA"),
        text_muted: Color::from_hex("7A9AA3"),
        accent: accent,
        border: Color::from_hex("FFFFFF"),
        hover: primary.with_alpha(0.25),
        danger: Color::from_hex("C0392B"),
        selection: primary.with_alpha(0.25),
        highlight: highlight,
        placeholder: primary.with_alpha(0.22),
        card: panel,
        grid: primary.with_alpha(0.15),
        overlay: Color::from_hex("000000").with_alpha(0.6),
        panel: panel,
        panel_text: Color::from_hex("FFFFFF"),
    }
}

inventory::submit! {
    ThemePreset {
        name: "Bishop",
        build: bishop_theme,
    }
}
