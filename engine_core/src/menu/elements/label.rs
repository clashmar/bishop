use super::element_kind::ElementKind;
use super::menu_element::{MenuElement, MenuElementKind, MenuEnabledWhen};
use crate::menu::layout::HorizontalAlign;
use crate::menu::runtime::RenderEnv;
use crate::menu::MenuAction;
use bishop::prelude::*;
use bishop::text::FontId;
use serde::{Deserialize, Serialize};
use widgets::{Label, LabelAlign, Widget};

/// Label element displaying text resolved from a text key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelElement {
    pub text_key: String,
    pub font_size: f32,
    /// Registered game font, or `None` for the embedded default.
    #[serde(default)]
    pub font: Option<FontId>,
    #[serde(default)]
    pub alignment: HorizontalAlign,
}

impl LabelElement {
    pub const KIND_NAME: &'static str = "Label";
}

impl Default for LabelElement {
    fn default() -> Self {
        Self {
            text_key: String::new(),
            font_size: 20.0,
            font: None,
            alignment: HorizontalAlign::default(),
        }
    }
}

impl ElementKind for LabelElement {
    fn kind_name(&self) -> &'static str {
        Self::KIND_NAME
    }

    fn default_rect(&self) -> bishop::Rect {
        bishop::Rect::new(0.0, 0.0, 0.2, 0.05)
    }

    fn wrap(&self, name: String) -> MenuElement {
        MenuElement {
            name,
            kind: MenuElementKind::Label(LabelElement::default()),
            rect: self.default_rect(),
            enabled: true,
            enabled_when: MenuEnabledWhen::Always,
            visible: true,
            z_order: 0,
            class: None,
            style_id: None,
        }
    }
}

impl LabelElement {
    pub(crate) fn render<C: BishopContext>(
        &self,
        ctx: &mut C,
        element: &MenuElement,
        screen_rect: Rect,
        _is_focused: bool,
        env: &mut RenderEnv<'_>,
    ) -> Option<MenuAction> {
        let display_text = env
            .text_manager
            .resolve_ui_text(env.text_id, &self.text_key);
        let font = self.font.and_then(|id| {
            env.font_manager.ensure_font(
                ctx,
                env.asset_registry,
                env.resource_source,
                id,
            )
        });
        let label_align = match self.alignment {
            HorizontalAlign::Left => LabelAlign::Left,
            HorizontalAlign::Center => LabelAlign::Center,
            HorizontalAlign::Right => LabelAlign::Right,
        };
        Label::new(screen_rect, display_text)
            .font_size(self.font_size)
            .font(font)
            .alignment(label_align)
            .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
            .show(ctx);
        None
    }
}
