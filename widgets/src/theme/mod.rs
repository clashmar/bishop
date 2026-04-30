pub mod visuals;

pub use visuals::{resolve, WidgetVisuals};

use crate::constants::colors;
use bishop::Color;

/// A collection of semantic color roles used by widgets and editor UI.
///
/// The default implementation captures the current hardcoded constant values,
/// so there is zero visual change at rest. Applications populate the active
/// theme from a config source (editor_config.ron, theme.ron) and push it
/// into the global `ACTIVE_THEME` static via `widgets::theme::set_theme()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub background: Color,
    pub surface: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub border: Color,
    pub hover: Color,
    pub danger: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: colors::DEFAULT_PRIMARY_COLOR,
            secondary: colors::DEFAULT_SECONDARY_COLOR,
            background: colors::DEFAULT_BACKGROUND_COLOR,
            surface: colors::DEFAULT_SURFACE_COLOR,
            text: colors::DEFAULT_TEXT_COLOR,
            text_muted: colors::DEFAULT_TEXT_MUTED_COLOR,
            accent: colors::DEFAULT_ACCENT_COLOR,
            border: colors::DEFAULT_BORDER_COLOR,
            hover: colors::DEFAULT_HOVER_COLOR,
            danger: colors::DEFAULT_DANGER_COLOR,
        }
    }
}
