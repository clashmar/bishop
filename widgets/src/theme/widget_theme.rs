use crate::widgets::Widget;
use bishop::Color;
use serde::{Deserialize, Serialize};

/// The canonical list of theme color fields with descriptions.
///
/// Struct definitions, `merge`, `apply`, [`each_color_field`], and
/// [`each_color_field_desc`] all derive from this list.
/// Update here when adding/removing fields — docs regenerate automatically.
#[doc(hidden)]
#[macro_export]
macro_rules! define_theme_colors {
    ($m:ident) => {
        $m!(primary, "Brand accent; interactive control fill");
        $m!(secondary, "Alternate accent for secondary emphasis");
        $m!(background, "Page-level background");
        $m!(surface, "Elevated surfaces above background");
        $m!(text, "Primary text for readability");
        $m!(text_muted, "Subdued text for secondary or disabled content");
        $m!(accent, "Emphasized accent for active or focused elements");
        $m!(border, "Outline color for widgets and containers");
        $m!(hover, "Hover or pressed overlay");
        $m!(danger, "Error, destructive action, or critical warning");
        $m!(selection, "Text-selection highlight background");
        $m!(
            highlight,
            "Transient highlight for active or matching elements"
        );
        $m!(placeholder, "Fill for placeholder or ghost content");
        $m!(overlay, "Scrim or backdrop for overlays and modals");
        $m!(panel, "Large surface for panels and sidebars");
        $m!(panel_text, "Text rendered on panel surfaces");
    };
}

/// Visits every theme color field by name, calling the caller-provided
/// macro `$m` once per field. Description is discarded.
///
/// Delegates to [`define_theme_colors`] so the field list stays in sync.
#[doc(hidden)]
#[macro_export]
macro_rules! each_color_field {
    ($cb:ident) => {
        macro_rules! __field_name_only {
            ($f:ident, $desc:literal) => {
                $cb!($f);
            };
        }
        $crate::define_theme_colors!(__field_name_only);
    };
}

/// Visits every theme color field, passing the field name and its
/// description to the caller-provided macro `$m`.
#[doc(hidden)]
#[macro_export]
macro_rules! each_color_field_desc {
    ($cb:ident) => {
        $crate::define_theme_colors!($cb);
    };
}

/// Per-widget theme overrides. Every field is `Option<Color>` — `None` defers
/// to the active theme, then to the hardcoded constant fallback.
///
/// Override individual fields with the struct update syntax:
/// ```ignore
/// WidgetTheme { background: Some(Color::RED), ..Default::default() }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WidgetTheme {
    pub primary: Option<Color>,
    pub secondary: Option<Color>,
    pub background: Option<Color>,
    pub surface: Option<Color>,
    pub text: Option<Color>,
    pub text_muted: Option<Color>,
    pub accent: Option<Color>,
    pub border: Option<Color>,
    pub hover: Option<Color>,
    pub danger: Option<Color>,
    pub selection: Option<Color>,
    pub highlight: Option<Color>,
    pub placeholder: Option<Color>,
    pub overlay: Option<Color>,
    pub panel: Option<Color>,
    pub panel_text: Option<Color>,
}

impl WidgetTheme {
    /// Merge instance overrides with theme-derived values.
    /// Instance wins where present; theme fills gaps.
    pub fn merge(self, other: Self) -> Self {
        let mut out = self;
        macro_rules! merge_one {
            ($f:ident) => {
                out.$f = out.$f.or(other.$f);
            };
        }
        each_color_field!(merge_one);
        out
    }

    /// Overlays non-`None` fields from `other` onto `self`.
    pub fn apply(&mut self, other: &Self) {
        macro_rules! apply_one {
            ($f:ident) => {
                if let Some(v) = other.$f {
                    self.$f = Some(v);
                }
            };
        }
        each_color_field!(apply_one);
    }
}

/// Resolves a color value from the priority chain:
/// `instance override → hardcoded constant`.
pub fn resolve(instance: Option<Color>, constant: Color) -> Color {
    instance.unwrap_or(constant)
}

/// Resolves a color from the three-level priority chain:
/// `instance override → theme mapping → hardcoded constant`.
pub fn resolve_with_theme(
    instance: Option<Color>,
    theme_slot: Option<Color>,
    constant: Color,
) -> Color {
    instance.or(theme_slot).unwrap_or(constant)
}

/// Resolves theme overrides for a widget type, applying matching style rules.
pub fn resolve_theme_for<T: Widget>(class: Option<&str>, id: Option<&str>) -> WidgetTheme {
    super::with_theme(|t| {
        let mut base = T::map_theme(t);
        t.apply_rules(T::widget_type(), class, id, &mut base);
        base
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_instance_wins_over_constant() {
        let result = resolve(Some(Color::RED), Color::BLUE);
        assert_eq!(result, Color::RED);
    }

    #[test]
    fn resolve_none_falls_back_to_constant() {
        let result = resolve(None, Color::BLUE);
        assert_eq!(result, Color::BLUE);
    }

    #[test]
    fn resolve_default_widget_overrides_all_none() {
        let v = WidgetTheme::default();
        macro_rules! check_none {
            ($f:ident) => {
                assert!(v.$f.is_none(), "{} should be None", stringify!($f));
            };
        }
        each_color_field!(check_none);
    }

    #[test]
    fn partial_override_leaves_other_fields_default() {
        let v = WidgetTheme {
            primary: Some(Color::RED),
            ..Default::default()
        };
        assert_eq!(v.primary, Some(Color::RED));
        assert!(v.background.is_none());
        assert!(v.text.is_none());
        assert!(v.hover.is_none());
    }

    #[test]
    fn resolve_with_theme_instance_wins_over_theme_and_constant() {
        let result = resolve_with_theme(Some(Color::RED), Some(Color::GREEN), Color::BLUE);
        assert_eq!(result, Color::RED);
    }

    #[test]
    fn resolve_with_theme_theme_wins_over_constant_when_instance_none() {
        let result = resolve_with_theme(None, Some(Color::GREEN), Color::BLUE);
        assert_eq!(result, Color::GREEN);
    }

    #[test]
    fn resolve_with_theme_constant_fallback_when_both_none() {
        let result = resolve_with_theme(None, None, Color::BLUE);
        assert_eq!(result, Color::BLUE);
    }

    #[test]
    fn apply_overrides_some_fields_leaves_others() {
        let mut base = WidgetTheme {
            background: Some(Color::RED),
            text: Some(Color::WHITE),
            ..Default::default()
        };
        let overrides = WidgetTheme {
            background: Some(Color::BLUE),
            ..Default::default()
        };
        base.apply(&overrides);
        assert_eq!(base.background, Some(Color::BLUE));
        assert_eq!(base.text, Some(Color::WHITE));
    }

    #[test]
    fn apply_all_overrides_take_effect() {
        let mut base = WidgetTheme::default();
        let overrides = WidgetTheme {
            background: Some(Color::RED),
            text: Some(Color::BLUE),
            border: Some(Color::GREEN),
            ..Default::default()
        };
        base.apply(&overrides);
        assert_eq!(base.background, Some(Color::RED));
        assert_eq!(base.text, Some(Color::BLUE));
        assert_eq!(base.border, Some(Color::GREEN));
    }
}
