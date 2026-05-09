use super::element_kind::ElementKind;
use super::menu_element::{MenuElement, MenuElementKind};
use crate::menu::runtime::RenderEnv;
use crate::prelude::MenuAction;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use widgets::*;

/// Decorative panel element styled via theme.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PanelElement;

impl ElementKind for PanelElement {
    fn kind_name(&self) -> &'static str {
        Self::KIND_NAME
    }

    fn default_rect(&self) -> bishop::Rect {
        bishop::Rect::new(0.0, 0.0, 0.3, 0.2)
    }

    fn wrap(&self, name: String) -> MenuElement {
        MenuElement {
            name,
            kind: MenuElementKind::Panel(PanelElement),
            rect: self.default_rect(),
            enabled: true,
            visible: true,
            z_order: 0,
            class: None,
            style_id: None,
        }
    }
}

impl PanelElement {
    pub const KIND_NAME: &'static str = "Panel";

    pub(crate) fn render<C: BishopContext>(
        &self,
        ctx: &mut C,
        element: &MenuElement,
        screen_rect: Rect,
        _is_focused: bool,
        _env: &mut RenderEnv<'_>,
    ) -> Option<MenuAction> {
        Panel::new(screen_rect)
            .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
            .show(ctx);
        None
    }
}
