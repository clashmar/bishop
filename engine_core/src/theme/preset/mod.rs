use widgets::theme::Theme;

pub mod bishop;
pub mod dark;
pub mod default;

pub const DEFAULT_PRESET_NAME: &str = "Default";

pub struct ThemePreset {
    pub name: &'static str,
    pub build: fn() -> Theme,
}

inventory::collect!(ThemePreset);

pub fn all_presets() -> Vec<&'static ThemePreset> {
    inventory::iter::<ThemePreset>().collect()
}

pub fn find_preset_by_name(name: &str) -> Option<&'static ThemePreset> {
    all_presets().into_iter().find(|p| p.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_at_least_two_presets() {
        assert!(all_presets().len() >= 2);
    }

    #[test]
    fn registry_names_are_unique() {
        let presets = all_presets();
        let names: Vec<_> = presets.iter().map(|p| p.name).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }
}
