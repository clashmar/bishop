use bishop::Color;
use widgets::constants::colors;
use widgets::theme::{StyleRule, StyleSelector, Theme, WidgetTheme, WidgetType};

use super::ThemePreset;

fn bishop_theme() -> Theme {
    let primary = Color::from_hex("22b1d0");
    let secondary = Color::from_hex("222034");
    let background = Color::from_hex("000000");
    let accent = Color::from_hex("d95763");
    let highlight = colors::DEFAULT_HIGHLIGHT_COLOR;
    let panel = Color::from_hex("7c9ea6");

    Theme {
        primary,
        secondary,
        background,
        surface: primary,
        text: Color::from_hex("E0E8EA"),
        text_muted: Color::from_hex("7A9AA3"),
        accent,
        border: Color::from_hex("FFFFFF"),
        hover: primary.with_alpha(0.25),
        danger: Color::from_hex("C0392B"),
        selection: primary.with_alpha(0.25),
        highlight,
        placeholder: primary.with_alpha(0.22),
        overlay: Color::from_hex("000000").with_alpha(0.6),
        panel,
        rules: vec![
            StyleRule {
                selector: StyleSelector::Type(WidgetType::Button),
                properties: WidgetTheme {
                    primary: Some(background),
                    ..WidgetTheme::default()
                },
            },
            StyleRule {
                selector: StyleSelector::Class("panel-text".into()),
                properties: WidgetTheme {
                    text: Some(Color::from_hex("FFFFFF")),
                    ..WidgetTheme::default()
                },
            },
        ],
    }
}

inventory::submit! {
    ThemePreset {
        name: "Bishop",
        build: bishop_theme,
    }
}
