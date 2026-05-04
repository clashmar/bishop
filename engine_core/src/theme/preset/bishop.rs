use bishop::Color;
use widgets::theme::Theme;

use super::ThemePreset;

fn bishop_theme() -> Theme {
    let primary = Color::from_hex("000000");
    let secondary = Color::from_hex("122D35");

    Theme {
        primary,
        secondary: Color::from_hex("0D3D47"),
        background: Color::from_hex("0B1E23"),
        surface: secondary,
        text: Color::from_hex("E0E8EA"),
        text_muted: Color::from_hex("7A9AA3"),
        accent: primary,
        border: Color::from_hex("1A3A42"),
        hover: Color::from_hex("1D4F5C"),
        danger: Color::from_hex("C0392B"),
        selection: primary.with_alpha(0.25),
        highlight: primary,
        placeholder: primary.with_alpha(0.22),
        card: secondary,
        grid: primary.with_alpha(0.15),
        overlay: Color::from_hex("000000").with_alpha(0.6),
        panel: primary,
        panel_text: Color::from_hex("FFFFFF"),
    }
}

inventory::submit! {
    ThemePreset {
        name: "Bishop",
        build: bishop_theme,
    }
}
