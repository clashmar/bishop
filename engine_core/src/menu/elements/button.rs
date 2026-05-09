use super::element_kind::ElementKind;
use super::menu_element::{MenuElement, MenuElementKind};
use crate::menu::menu_builder::MenuAction;
use crate::menu::runtime::RenderEnv;
use crate::menu::{NavTargets, Navigable};
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use widgets::*;

/// Button element that triggers an action when clicked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonElement {
    pub text_key: String,
    pub action: MenuAction,
    pub font_size: f32,
    pub nav_targets: NavTargets,
}

impl Default for ButtonElement {
    fn default() -> Self {
        Self {
            text_key: String::new(),
            action: MenuAction::CloseMenu,
            font_size: 20.0,
            nav_targets: NavTargets::default(),
        }
    }
}

impl ElementKind for ButtonElement {
    fn kind_name(&self) -> &'static str {
        Self::KIND_NAME
    }

    fn default_rect(&self) -> bishop::Rect {
        bishop::Rect::new(0.0, 0.0, 0.2, 0.06)
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn wrap(&self, name: String) -> MenuElement {
        MenuElement {
            name,
            kind: MenuElementKind::Button(ButtonElement::default()),
            rect: self.default_rect(),
            enabled: true,
            visible: true,
            z_order: 0,
            class: None,
            style_id: None,
        }
    }
}

impl Navigable for ButtonElement {
    fn nav_targets(&self) -> &NavTargets {
        &self.nav_targets
    }

    fn nav_targets_mut(&mut self) -> &mut NavTargets {
        &mut self.nav_targets
    }

    fn from_element(el: &MenuElement) -> Option<&Self> {
        match &el.kind {
            MenuElementKind::Button(b) => Some(b),
            _ => None,
        }
    }

    fn wrap_into_element(self) -> MenuElementKind {
        MenuElementKind::Button(self)
    }
}

impl ButtonElement {
    pub const KIND_NAME: &'static str = "Button";

    pub(crate) fn render<C: BishopContext>(
        &self,
        ctx: &mut C,
        element: &MenuElement,
        screen_rect: Rect,
        is_focused: bool,
        env: &mut RenderEnv<'_>,
    ) -> Option<MenuAction> {
        let display_text = env
            .text_manager
            .resolve_ui_text(env.text_id, &self.text_key);
        let widget = Button::new(screen_rect, &display_text)
            .blocked(!element.enabled)
            .focused(is_focused)
            .apply_selectors(element.class.as_deref(), element.style_id.as_deref());
        if widget.show(ctx) {
            Some(self.action.clone())
        } else {
            None
        }
    }
}
