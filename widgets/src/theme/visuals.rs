use crate::theme::Theme;
use bishop::Color;

/// Per-widget visual overrides. Every field is `Option<Color>` — `None` defers
/// to the active theme, then to the hardcoded constant fallback.
///
/// Override individual fields with the struct update syntax:
/// ```ignore
/// WidgetVisuals { background: Some(Color::RED), ..Default::default() }
/// ```
#[derive(Clone, Copy, Debug, Default)]
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
        }
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
    fn theme_visuals(theme: &Theme) -> WidgetVisuals;
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
        assert!(v.primary.is_none());
        assert!(v.secondary.is_none());
        assert!(v.background.is_none());
        assert!(v.surface.is_none());
        assert!(v.text.is_none());
        assert!(v.text_muted.is_none());
        assert!(v.accent.is_none());
        assert!(v.border.is_none());
        assert!(v.hover.is_none());
        assert!(v.danger.is_none());
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
}
