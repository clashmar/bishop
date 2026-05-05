use bishop::Color;
use widgets::theme::{StyleRule, StyleSelector, Theme, WidgetTheme};

use super::ThemePreset;

fn default_theme() -> Theme {
    Theme {
        rules: vec![StyleRule {
            selector: StyleSelector::Class("panel-text".into()),
            properties: WidgetTheme {
                text: Some(Color::BLACK),
                ..WidgetTheme::default()
            },
        }],
        ..Theme::default()
    }
}

inventory::submit! {
    ThemePreset {
        name: "Default",
        build: default_theme,
    }
}
