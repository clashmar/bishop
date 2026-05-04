use crate::theme::{Theme, WidgetType};
use bishop::Color;
use serde::{Deserialize, Serialize};

/// Defines the list of color fields on [`WidgetVisuals`] and [`Theme`].
/// Every field is visited by the caller-provided macro `$m`.
///
/// When you add/remove/rename a color field on the struct, update this
/// macro *and* the struct definition — they must stay in sync.
#[doc(hidden)]
#[macro_export]
macro_rules! each_color_field {
    ($m:ident) => {
        $m!(primary);
        $m!(secondary);
        $m!(background);
        $m!(surface);
        $m!(text);
        $m!(text_muted);
        $m!(accent);
        $m!(border);
        $m!(hover);
        $m!(danger);
        $m!(selection);
        $m!(highlight);
        $m!(placeholder);
        $m!(card);
        $m!(grid);
        $m!(overlay);
        $m!(panel);
        $m!(panel_text);
    };
}

/// Per-widget visual overrides. Every field is `Option<Color>` — `None` defers
/// to the active theme, then to the hardcoded constant fallback.
///
/// Override individual fields with the struct update syntax:
/// ```ignore
/// WidgetVisuals { background: Some(Color::RED), ..Default::default() }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WidgetVisuals {
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
    pub card: Option<Color>,
    pub grid: Option<Color>,
    pub overlay: Option<Color>,
    pub panel: Option<Color>,
    pub panel_text: Option<Color>,
}

impl WidgetVisuals {
    /// Merge instance overrides with theme-derived values.
    /// Instance wins where present; theme fills gaps.
    pub fn merge(self, theme: Self) -> Self {
        Self {
            primary: self.primary.or(theme.primary),
            secondary: self.secondary.or(theme.secondary),
            background: self.background.or(theme.background),
            surface: self.surface.or(theme.surface),
            text: self.text.or(theme.text),
            text_muted: self.text_muted.or(theme.text_muted),
            accent: self.accent.or(theme.accent),
            border: self.border.or(theme.border),
            hover: self.hover.or(theme.hover),
            danger: self.danger.or(theme.danger),
            selection: self.selection.or(theme.selection),
            highlight: self.highlight.or(theme.highlight),
            placeholder: self.placeholder.or(theme.placeholder),
            card: self.card.or(theme.card),
            grid: self.grid.or(theme.grid),
            overlay: self.overlay.or(theme.overlay),
            panel: self.panel.or(theme.panel),
            panel_text: self.panel_text.or(theme.panel_text),
        }
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

/// Maps a `Theme` to `WidgetVisuals` for a specific widget type.
///
/// Each widget implements this trait to declare which `Theme` roles
/// it reads and how they map to its visual fields.
pub trait WidgetThemeMapper {
    /// Returns the widget type kind for style rule matching.
    fn type_kind() -> WidgetType;
    /// Maps theme roles to WidgetVisuals for this widget type.
    fn theme_visuals(theme: &Theme) -> WidgetVisuals;
}

/// Resolves theme visuals for a widget type, applying matching style rules.
pub fn themed_visuals_for<T: WidgetThemeMapper>(
    class: Option<&str>,
    id: Option<&str>,
) -> WidgetVisuals {
    super::with_theme(|t| {
        let mut base = T::theme_visuals(t);
        t.apply_rules(T::type_kind(), class, id, &mut base);
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
    fn resolve_default_widget_visuals_all_none() {
        let v = WidgetVisuals::default();
        macro_rules! check_none {
            ($f:ident) => {
                assert!(v.$f.is_none(), "{} should be None", stringify!($f));
            };
        }
        each_color_field!(check_none);
    }

    #[test]
    fn partial_override_leaves_other_fields_default() {
        let v = WidgetVisuals {
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
        let mut base = WidgetVisuals {
            background: Some(Color::RED),
            text: Some(Color::WHITE),
            ..Default::default()
        };
        let overrides = WidgetVisuals {
            background: Some(Color::BLUE),
            ..Default::default()
        };
        base.apply(&overrides);
        assert_eq!(base.background, Some(Color::BLUE));
        assert_eq!(base.text, Some(Color::WHITE));
    }

    #[test]
    fn apply_all_overrides_take_effect() {
        let mut base = WidgetVisuals::default();
        let overrides = WidgetVisuals {
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
