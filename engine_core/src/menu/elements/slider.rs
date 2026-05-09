use super::element_kind::ElementKind;
use super::menu_element::{MenuElement, MenuElementKind};
use crate::menu::runtime::RenderEnv;
use crate::menu::*;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use widgets::WidgetId;
use widgets::*;

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

impl ElementKind for SliderElement {
    fn kind_name(&self) -> &'static str {
        Self::KIND_NAME
    }

    fn default_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, 0.4, 0.06)
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn wrap(&self, name: String) -> MenuElement {
        MenuElement {
            name,
            kind: MenuElementKind::Slider(SliderElement::default()),
            rect: self.default_rect(),
            enabled: true,
            visible: true,
            z_order: 0,
            class: None,
            style_id: None,
        }
    }
}

impl SliderElement {
    pub const KIND_NAME: &'static str = "Slider";

    pub(crate) fn render<C: BishopContext>(
        &self,
        ctx: &mut C,
        element: &MenuElement,
        screen_rect: Rect,
        is_focused: bool,
        env: &mut RenderEnv<'_>,
    ) -> Option<MenuAction> {
        let value = env
            .slider_values
            .get(&self.key)
            .copied()
            .unwrap_or(self.default_value);
        let display_text = env
            .text_manager
            .resolve_ui_text(env.text_id, &self.text_key);
        let (new_value, state) =
            Slider::new(self.widget_id, screen_rect, self.min, self.max, value)
                .label(&display_text)
                .focused(is_focused)
                .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
                .show(ctx);
        if !matches!(state, SliderState::Unchanged) {
            env.slider_values.insert(self.key.clone(), new_value);
            push_slider_event(self.key.clone(), new_value);
        }
        None
    }
}
