use bishop::Color;
use widgets::theme::Theme;

pub fn default() -> Theme {
    Theme::default()
}

pub fn dark() -> Theme {
    Theme {
        background: Color::new(0.05, 0.05, 0.08, 1.0),
        surface: Color::new(0.10, 0.10, 0.14, 1.0),
        text: Color::new(0.90, 0.90, 0.95, 1.0),
        text_muted: Color::new(0.55, 0.55, 0.60, 1.0),
        border: Color::new(0.25, 0.25, 0.30, 1.0),
        hover: Color::new(0.20, 0.20, 0.28, 1.0),
        ..Theme::default()
    }
}

pub fn high_contrast() -> Theme {
    Theme {
        background: Color::new(0.0, 0.0, 0.0, 1.0),
        surface: Color::new(0.15, 0.15, 0.15, 1.0),
        text: Color::new(1.0, 1.0, 1.0, 1.0),
        text_muted: Color::new(0.75, 0.75, 0.75, 1.0),
        accent: Color::new(1.0, 0.85, 0.0, 1.0),
        border: Color::new(0.5, 0.5, 0.5, 1.0),
        hover: Color::new(0.3, 0.3, 0.3, 1.0),
        danger: Color::new(1.0, 0.2, 0.2, 1.0),
        ..Theme::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_valid_color(c: Color, name: &str) {
        assert!(!c.r.is_nan(), "{name}.r is NaN");
        assert!(!c.g.is_nan(), "{name}.g is NaN");
        assert!(!c.b.is_nan(), "{name}.b is NaN");
        assert!(!c.a.is_nan(), "{name}.a is NaN");
    }

    fn assert_valid_theme(theme: &Theme, label: &str) {
        assert_valid_color(theme.primary, &format!("{label}.primary"));
        assert_valid_color(theme.secondary, &format!("{label}.secondary"));
        assert_valid_color(theme.background, &format!("{label}.background"));
        assert_valid_color(theme.surface, &format!("{label}.surface"));
        assert_valid_color(theme.text, &format!("{label}.text"));
        assert_valid_color(theme.text_muted, &format!("{label}.text_muted"));
        assert_valid_color(theme.accent, &format!("{label}.accent"));
        assert_valid_color(theme.border, &format!("{label}.border"));
        assert_valid_color(theme.hover, &format!("{label}.hover"));
        assert_valid_color(theme.danger, &format!("{label}.danger"));
    }

    #[test]
    fn default_preset_has_valid_colors() {
        assert_valid_theme(&default(), "default");
    }

    #[test]
    fn dark_preset_has_valid_colors() {
        assert_valid_theme(&dark(), "dark");
    }

    #[test]
    fn high_contrast_preset_has_valid_colors() {
        assert_valid_theme(&high_contrast(), "high_contrast");
    }
}
