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

/// Resolves a color value from the priority chain:
/// `instance override → hardcoded constant`.
pub fn resolve(instance: Option<Color>, constant: Color) -> Color {
    instance.unwrap_or(constant)
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
}
