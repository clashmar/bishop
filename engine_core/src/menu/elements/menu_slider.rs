use crate::menu::*;
use serde::{Deserialize, Serialize};
use widgets::WidgetId;

/// Slider element for adjusting a numeric value within a bounded range.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SliderElement {
    /// Label text key resolved via TextManager.
    pub text_key: String,
    /// Value identifier used to store and retrieve the setting (e.g. `"master_volume"`).
    pub key: String,
    /// Minimum value of the slider range.
    pub min: f32,
    /// Maximum value of the slider range.
    pub max: f32,
    /// Increment applied when navigating left or right with the keyboard.
    pub step: f32,
    /// Value used when no saved setting is present.
    pub default_value: f32,
    /// Stable widget identifier within the session; not persisted across saves.
    #[serde(skip)]
    pub widget_id: WidgetId,
    /// Navigation targets for each direction.
    pub nav_targets: NavTargets,
}

impl Navigable for SliderElement {
    fn nav_targets(&self) -> &NavTargets {
        &self.nav_targets
    }

    fn nav_targets_mut(&mut self) -> &mut NavTargets {
        &mut self.nav_targets
    }

    fn from_element(el: &MenuElement) -> Option<&Self> {
        match &el.kind {
            MenuElementKind::Slider(s) => Some(s),
            _ => None,
        }
    }

    fn wrap_into_element(self) -> MenuElementKind {
        MenuElementKind::Slider(self)
    }
}
