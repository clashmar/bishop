pub mod visuals;

pub use visuals::{resolve, resolve_with_theme, WidgetThemeMapper, WidgetVisuals};

use crate::constants::colors;
use bishop::Color;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// A collection of semantic color roles.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    /// Primary accent color.
    pub primary: Color,
    /// Secondary accent color.
    pub secondary: Color,
    /// Deepest background tone.
    pub background: Color,
    /// Raised surface tone.
    pub surface: Color,
    /// Primary text color.
    pub text: Color,
    /// Diminished text for labels, hints, disabled state.
    pub text_muted: Color,
    /// Accent highlight (selections, check marks, active indicators).
    pub accent: Color,
    /// Border and outline color.
    pub border: Color,
    /// Hover highlight color.
    pub hover: Color,
    /// Danger/error/destructive-action color.
    pub danger: Color,
    /// Selection highlight background.
    pub selection: Color,
    /// Active element highlight (marquee, entity outline, active prefab).
    pub highlight: Color,
    /// Room editor ghost stamp fill.
    pub placeholder: Color,
    /// Card background (prefab/resource cards).
    pub card: Color,
    /// Grid line color.
    pub grid: Color,
    /// Overlay base (toast, tooltip, modal backdrop — alpha applied per-context).
    pub overlay: Color,
    /// Panel background (menu bar, side panels, dropdowns).
    pub panel: Color,
    /// Text on panel backgrounds (contrasts with panel).
    pub panel_text: Color,
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
            selection: colors::DEFAULT_SELECTION_COLOR,
            highlight: colors::DEFAULT_HIGHLIGHT_COLOR,
            placeholder: colors::DEFAULT_PLACEHOLDER_COLOR,
            card: colors::DEFAULT_CARD_COLOR,
            grid: colors::DEFAULT_GRID_COLOR,
            overlay: colors::DEFAULT_OVERLAY_COLOR,
            panel: colors::DEFAULT_PANEL_COLOR,
            panel_text: colors::DEFAULT_PANEL_TEXT_COLOR,
        }
    }
}

pub static ACTIVE_THEME: Lazy<RwLock<Theme>> = Lazy::new(|| RwLock::new(Theme::default()));

pub fn set_theme(theme: Theme) {
    *ACTIVE_THEME.write().expect("ACTIVE_THEME lock poisoned") = theme;
}

pub fn with_theme<R>(f: impl FnOnce(&Theme) -> R) -> R {
    let guard = ACTIVE_THEME.read().expect("ACTIVE_THEME lock poisoned");
    f(&guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_ron_roundtrip() {
        let original = Theme::default();
        let ron = ron::to_string(&original).unwrap();
        let loaded: Theme = ron::from_str(&ron).unwrap();
        assert_eq!(original, loaded);
    }

    #[test]
    fn set_theme_updates_active_theme() {
        let original = Theme::default();
        set_theme(original);
        let read_back = with_theme(|t| *t);
        assert_eq!(original, read_back);

        let dark = Theme {
            background: Color::new(0.05, 0.05, 0.08, 1.0),
            surface: Color::new(0.10, 0.10, 0.14, 1.0),
            ..Theme::default()
        };
        set_theme(dark);
        let read_back = with_theme(|t| *t);
        assert_eq!(dark, read_back);
        assert_ne!(original, read_back);
    }
}
