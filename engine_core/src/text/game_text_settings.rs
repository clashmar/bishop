use bishop::text::FontId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameTextSettings {
    /// The default registered font for game text without an element override.
    pub default_font: Option<FontId>,
}

impl GameTextSettings {
    /// Returns an element override or the game default font.
    pub fn font_for_element(&self, override_font: Option<FontId>) -> Option<FontId> {
        override_font.or(self.default_font)
    }
}

#[cfg(test)]
mod tests {
    use super::GameTextSettings;
    use bishop::text::FontId;

    #[test]
    fn font_for_element_uses_override_before_game_default() {
        let settings = GameTextSettings {
            default_font: Some(FontId(3)),
        };

        assert_eq!(settings.font_for_element(None), Some(FontId(3)));
        assert_eq!(settings.font_for_element(Some(FontId(8))), Some(FontId(8)));
        assert_eq!(
            GameTextSettings::default().font_for_element(None),
            None
        );
    }
}
