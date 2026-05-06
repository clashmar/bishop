use super::element_kind::ElementKind;
use super::menu_element::{MenuElement, MenuElementKind};
use crate::menu::*;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use widgets::*;

/// Element that arranges its children using layout rules.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LayoutGroupElement {
    pub layout: LayoutConfig,
    pub children: Vec<LayoutChild>,
    pub nav_targets: NavTargets,
}

impl Navigable for LayoutGroupElement {
    fn nav_targets(&self) -> &NavTargets {
        &self.nav_targets
    }

    fn nav_targets_mut(&mut self) -> &mut NavTargets {
        &mut self.nav_targets
    }

    fn from_element(el: &MenuElement) -> Option<&Self> {
        match &el.kind {
            MenuElementKind::LayoutGroup(group) => Some(group),
            _ => None,
        }
    }

    fn wrap_into_element(self) -> MenuElementKind {
        MenuElementKind::LayoutGroup(self)
    }
}

impl ElementKind for LayoutGroupElement {
    fn kind_name(&self) -> &'static str {
        Self::KIND_NAME
    }

    fn default_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, 0.5, 0.3)
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn wrap(&self, name: String) -> MenuElement {
        MenuElement {
            name,
            kind: MenuElementKind::LayoutGroup(LayoutGroupElement::default()),
            rect: self.default_rect(),
            enabled: true,
            visible: true,
            z_order: 0,
            class: None,
            style_id: None,
        }
    }
}

impl LayoutGroupElement {
    pub const KIND_NAME: &'static str = "Layout Group";

    pub(crate) fn render<C: BishopContext>(
        &self,
        ctx: &mut C,
        element: &MenuElement,
        screen_rect: Rect,
        _is_focused: bool,
        env: &mut crate::menu::runtime::RenderEnv<'_>,
    ) -> Option<crate::menu::menu_builder::MenuAction> {
        for child in self.children.iter().filter(|c| !c.managed) {
            if !child.element.visible {
                continue;
            }
            if let MenuElementKind::Panel(_) = &child.element.kind {
                Panel::new(screen_rect)
                    .apply_selectors(
                        child.element.class.as_deref(),
                        child.element.style_id.as_deref(),
                    )
                    .show(ctx);
            }
        }

        let resolved = resolve_layout(self, element.rect);
        let mut focusable_idx = 0;
        let mut action = None;
        let element_index = env.current_element_index;

        for (child, rect) in self.children.iter().zip(resolved.iter()) {
            if !child.element.visible || !child.managed {
                continue;
            }

            let child_screen = normalized_rect_to_screen(*rect, env.canvas_origin, env.canvas_size);
            let is_focused =
                env.focus.node == element_index && env.focus.child == Some(focusable_idx);

            let child_action = match &child.element.kind {
                MenuElementKind::Label(l) => {
                    l.render(ctx, &child.element, child_screen, false, env)
                }
                MenuElementKind::Button(b) => {
                    b.render(ctx, &child.element, child_screen, is_focused, env)
                }
                MenuElementKind::Panel(p) => {
                    p.render(ctx, &child.element, child_screen, false, env)
                }
                MenuElementKind::Slider(s) => {
                    s.render(ctx, &child.element, child_screen, is_focused, env)
                }
                _ => None,
            };

            if child_action.is_some() {
                action = child_action;
            }

            if matches!(
                child.element.kind,
                MenuElementKind::Button(_) | MenuElementKind::Slider(_)
            ) && child.element.enabled
            {
                focusable_idx += 1;
            }
        }

        action
    }
}

/// A child element within a layout group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutChild {
    pub element: MenuElement,
    /// When true, position is computed from layout rules.
    /// When false, rect is relative to group origin but not subject to layout.
    pub managed: bool,
}
