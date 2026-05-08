use crate::prelude::constants::colors;
use bishop::Color;
use widgets::{
    theme::{StyleRule, StyleSelector, Theme, WidgetTheme},
    WidgetType,
};

use super::{ThemePreset, DEFAULT_PRESET_NAME};

fn default_theme() -> Theme {
    Theme {
        rules: vec![
            StyleRule {
                selector: StyleSelector::Type(WidgetType::Button),
                properties: WidgetTheme {
                    primary: Some(colors::DEFAULT_BACKGROUND_COLOR),
                    ..WidgetTheme::default()
                },
            },
            StyleRule {
                selector: StyleSelector::Class("panel-text".into()),
                properties: WidgetTheme {
                    text: Some(Color::BLACK),
                    ..WidgetTheme::default()
                },
            },
        ],
        ..Theme::default()
    }
}

inventory::submit! {
    ThemePreset {
        name: DEFAULT_PRESET_NAME,
        build: default_theme,
    }
}
