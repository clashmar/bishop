pub mod gui_constants;
pub mod inspector;
pub mod menu_bar;
pub mod modals;
pub mod mode_selector;
pub mod panels;
pub mod prompts;
pub mod properties;
pub mod text_input;
pub mod widgets;

use crate::constants::colors;
use crate::gui::gui_constants::classes;
use bishop::Color;
use engine_core::{
    theme::{with_theme, WidgetTheme},
};
use ::widgets::WidgetType;

pub fn panel_text_color() -> Color {
    with_theme(|t| {
        let mut base = WidgetTheme {
            text: Some(t.text),
            ..Default::default()
        };
        t.apply_rules(
            WidgetType::Label,
            Some(classes::PANEL_TEXT),
            None,
            &mut base,
        );
        base.text.unwrap_or(colors::DEFAULT_TEXT_COLOR)
    })
}
