pub mod widget_theme;

pub use widget_theme::{
    resolve, resolve_with_theme, resolve_theme_for, WidgetTheme,
};

use crate::constants::colors;
use bishop::Color;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// A collection of semantic color roles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Style rules applied during widget rendering.
    #[serde(default)]
    pub rules: Vec<StyleRule>,
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
            rules: Vec::new(),
        }
    }
}

/// Identifies a widget type for style rule targeting.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize,
    strum_macros::EnumString, strum_macros::Display, strum_macros::VariantNames,
)]
pub enum WidgetType {
    Button,
    Slider,
    Checkbox,
    TextInput,
    NumberInput,
    Dropdown,
    ContextMenu,
    ColorInput,
    Stepper,
    ScrollableArea,
}

/// A selector that targets widgets for style rule application.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StyleSelector {
    /// Targets all widgets of a given type.
    Type(WidgetType),
    /// Targets widgets whose `class_name` matches.
    Class(String),
    /// Targets a specific widget by `style_id`.
    Id(String),
}

impl Default for StyleSelector {
    fn default() -> Self {
        StyleSelector::Type(WidgetType::Button)
    }
}

#[allow(dead_code)]
impl StyleSelector {
    pub(crate) fn specificity_tier(&self) -> u8 {
        match self {
            StyleSelector::Type(_) => 1,
            StyleSelector::Class(_) => 10,
            StyleSelector::Id(_) => 100,
        }
    }

    pub(crate) fn matches(
        &self,
        widget_type: WidgetType,
        class: Option<&str>,
        id: Option<&str>,
    ) -> bool {
        match self {
            StyleSelector::Type(t) => *t == widget_type,
            StyleSelector::Class(c) => class == Some(c.as_str()),
            StyleSelector::Id(i) => id == Some(i.as_str()),
        }
    }
}

/// A single style rule mapping a selector to visual overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StyleRule {
    pub selector: StyleSelector,
    pub properties: WidgetTheme,
}

impl Theme {
    /// Applies matching style rules to `base` in priority order.
    /// Three-pass iteration: type (lowest) → class → id (highest).
    /// No allocation, no sorting — O(n) where n = rules.len().
    pub fn apply_rules(
        &self,
        widget_type: WidgetType,
        class: Option<&str>,
        id: Option<&str>,
        base: &mut WidgetTheme,
    ) {
        for rule in &self.rules {
            if let StyleSelector::Type(t) = &rule.selector
                && *t == widget_type
            {
                base.apply(&rule.properties);
            }
        }
        for rule in &self.rules {
            if let StyleSelector::Class(c) = &rule.selector
                && class == Some(c.as_str())
            {
                base.apply(&rule.properties);
            }
        }
        for rule in &self.rules {
            if let StyleSelector::Id(i) = &rule.selector
                && id == Some(i.as_str())
            {
                base.apply(&rule.properties);
            }
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

/// Returns a clone of the active theme.
pub fn get_theme() -> Theme {
    ACTIVE_THEME.read().expect("ACTIVE_THEME lock poisoned").clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WidgetBase;

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
        set_theme(original.clone());
        let read_back = with_theme(|t| t.clone());
        assert_eq!(original, read_back);

        let dark = Theme {
            background: Color::new(0.05, 0.05, 0.08, 1.0),
            surface: Color::new(0.10, 0.10, 0.14, 1.0),
            ..Theme::default()
        };
        set_theme(dark.clone());
        let read_back = with_theme(|t| t.clone());
        assert_eq!(dark, read_back);
        assert_ne!(original, read_back);
    }

    #[test]
    fn selector_type_matches_widget_type() {
        let sel = StyleSelector::Type(WidgetType::Button);
        assert!(sel.matches(WidgetType::Button, None, None));
        assert!(!sel.matches(WidgetType::Slider, None, None));
    }

    #[test]
    fn selector_class_matches_class_name() {
        let sel = StyleSelector::Class("danger".into());
        assert!(sel.matches(WidgetType::Button, Some("danger"), None));
        assert!(!sel.matches(WidgetType::Button, Some("hero"), None));
        assert!(!sel.matches(WidgetType::Button, None, None));
    }

    #[test]
    fn selector_id_matches_style_id() {
        let sel = StyleSelector::Id("confirm-btn".into());
        assert!(sel.matches(WidgetType::Button, None, Some("confirm-btn")));
        assert!(!sel.matches(WidgetType::Button, None, Some("other")));
        assert!(!sel.matches(WidgetType::Button, None, None));
    }

    #[test]
    fn selector_specificity_tiers() {
        assert_eq!(
            StyleSelector::Type(WidgetType::Button).specificity_tier(),
            1
        );
        assert_eq!(StyleSelector::Class("x".into()).specificity_tier(), 10);
        assert_eq!(StyleSelector::Id("x".into()).specificity_tier(), 100);
    }

    #[test]
    fn apply_rules_empty_returns_base_unchanged() {
        let theme = Theme::default();
        let mut base = WidgetTheme {
            background: Some(Color::RED),
            ..Default::default()
        };
        let expected_background = base.background;
        theme.apply_rules(WidgetType::Button, None, None, &mut base);
        assert_eq!(base.background, expected_background);
    }

    #[test]
    fn apply_rules_type_rule_overrides_base() {
        let mut theme = Theme::default();
        theme.rules.push(StyleRule {
            selector: StyleSelector::Type(WidgetType::Button),
            properties: WidgetTheme {
                background: Some(Color::BLUE),
                ..Default::default()
            },
        });
        let mut base = WidgetTheme {
            background: Some(Color::RED),
            text: Some(Color::WHITE),
            ..Default::default()
        };
        theme.apply_rules(WidgetType::Button, None, None, &mut base);
        assert_eq!(base.background, Some(Color::BLUE));
        assert_eq!(base.text, Some(Color::WHITE));
    }

    #[test]
    fn apply_rules_class_overrides_type() {
        let mut theme = Theme::default();
        theme.rules.push(StyleRule {
            selector: StyleSelector::Type(WidgetType::Button),
            properties: WidgetTheme {
                background: Some(Color::BLUE),
                ..Default::default()
            },
        });
        theme.rules.push(StyleRule {
            selector: StyleSelector::Class("danger".into()),
            properties: WidgetTheme {
                background: Some(Color::RED),
                ..Default::default()
            },
        });
        let mut base = WidgetTheme {
            background: Some(Color::GREEN),
            ..Default::default()
        };
        theme.apply_rules(WidgetType::Button, Some("danger"), None, &mut base);
        assert_eq!(base.background, Some(Color::RED));
    }

    #[test]
    fn apply_rules_id_overrides_class_and_type() {
        let mut theme = Theme::default();
        theme.rules.push(StyleRule {
            selector: StyleSelector::Type(WidgetType::Button),
            properties: WidgetTheme {
                background: Some(Color::BLUE),
                ..Default::default()
            },
        });
        theme.rules.push(StyleRule {
            selector: StyleSelector::Class("danger".into()),
            properties: WidgetTheme {
                background: Some(Color::RED),
                ..Default::default()
            },
        });
        theme.rules.push(StyleRule {
            selector: StyleSelector::Id("confirm".into()),
            properties: WidgetTheme {
                background: Some(Color::YELLOW),
                ..Default::default()
            },
        });
        let mut base = WidgetTheme {
            background: Some(Color::GREEN),
            ..Default::default()
        };
        theme.apply_rules(
            WidgetType::Button,
            Some("danger"),
            Some("confirm"),
            &mut base,
        );
        assert_eq!(base.background, Some(Color::YELLOW));
    }

    #[test]
    fn themed_visuals_respects_type_rule() {
        let mut theme = Theme::default();
        theme.background = Color::RED;
        theme.rules.push(StyleRule {
            selector: StyleSelector::Type(WidgetType::Button),
            properties: WidgetTheme {
                background: Some(Color::BLUE),
                ..Default::default()
            },
        });
        set_theme(theme);

        struct TestButton {
            base: WidgetBase,
        }
        impl crate::widgets::Widget for TestButton {
            fn widget_type() -> WidgetType { WidgetType::Button }
            fn base_mut(&mut self) -> &mut WidgetBase { &mut self.base }
            fn map_theme(theme: &Theme) -> WidgetTheme {
                WidgetTheme { background: Some(theme.background), ..Default::default() }
            }
        }

        let visuals = resolve_theme_for::<TestButton>(None, None);
        // Rule overrides base theme mapping
        assert_eq!(visuals.background, Some(Color::BLUE));
    }

    #[test]
    fn themed_visuals_class_overrides_type_rule() {
        let mut theme = Theme::default();
        theme.rules.push(StyleRule {
            selector: StyleSelector::Type(WidgetType::Button),
            properties: WidgetTheme {
                background: Some(Color::BLUE),
                ..Default::default()
            },
        });
        theme.rules.push(StyleRule {
            selector: StyleSelector::Class("danger".into()),
            properties: WidgetTheme {
                background: Some(Color::RED),
                ..Default::default()
            },
        });
        set_theme(theme);

        struct TestButton {
            base: WidgetBase,
        }
        impl crate::widgets::Widget for TestButton {
            fn widget_type() -> WidgetType { WidgetType::Button }
            fn base_mut(&mut self) -> &mut WidgetBase { &mut self.base }
            fn map_theme(theme: &Theme) -> WidgetTheme {
                WidgetTheme { background: Some(theme.background), ..Default::default() }
            }
        }

        let visuals = resolve_theme_for::<TestButton>(Some("danger"), None);
        assert_eq!(visuals.background, Some(Color::RED));
    }

    #[test]
    fn themed_visuals_non_matching_type_skips_rule() {
        let mut theme = Theme::default();
        theme.background = Color::GREEN;
        theme.rules.push(StyleRule {
            selector: StyleSelector::Type(WidgetType::Button),
            properties: WidgetTheme {
                background: Some(Color::BLUE),
                ..Default::default()
            },
        });
        set_theme(theme);

        struct TestSlider {
            base: WidgetBase,
        }
        impl crate::widgets::Widget for TestSlider {
            fn widget_type() -> WidgetType { WidgetType::Slider }
            fn base_mut(&mut self) -> &mut WidgetBase { &mut self.base }
            fn map_theme(theme: &Theme) -> WidgetTheme {
                WidgetTheme { background: Some(theme.background), ..Default::default() }
            }
        }

        let visuals = resolve_theme_for::<TestSlider>(None, None);
        // Rule targets Button, not Slider — base theme mapping passes through
        assert_eq!(visuals.background, Some(Color::GREEN));
    }
}
