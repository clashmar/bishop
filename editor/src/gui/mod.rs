// editor/src/gui/mod.rs
pub mod gui_constants;
pub mod inspector;
pub mod menu_bar;
pub mod menu_widgets;
pub mod modals;
pub mod mode_selector;
pub mod panels;
pub mod prompts;

use crate::constants::colors;
use crate::gui::gui_constants::classes;
use bishop::Color;
use engine_core::{theme::{WidgetTheme, with_theme}, ui::WidgetType};

/// Returns the text color for text rendered on panel-colored surfaces,
/// resolved through the `panel-text` class style rule.
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
