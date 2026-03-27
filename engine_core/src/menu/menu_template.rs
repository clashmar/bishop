use crate::menu::*;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};

/// Serializable menu definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuTemplate {
    pub id: String,
    pub background: MenuBackground,
    pub elements: Vec<MenuElement>,
    pub mode: MenuMode,
}

impl MenuTemplate {
    /// Creates a new menu template.
    pub fn new(id: String) -> Self {
        Self {
            id,
            background: MenuBackground::default(),
            elements: Vec::new(),
            mode: MenuMode::Paused,
        }
    }

    /// Renders the menu background over the given viewport rect.
    pub fn render_background<C: BishopContext>(&self, ctx: &mut C, viewport: Rect) {
        match self.background {
            MenuBackground::None => {}
            MenuBackground::SolidColor(color) => {
                ctx.draw_rectangle(viewport.x, viewport.y, viewport.w, viewport.h, color);
            }
            MenuBackground::Dimmed(alpha) => {
                ctx.draw_rectangle(
                    viewport.x,
                    viewport.y,
                    viewport.w,
                    viewport.h,
                    Color::new(0.0, 0.0, 0.0, alpha),
                );
            }
        }
    }

    /// Returns element indices sorted by z_order (stable, ascending).
    pub fn sorted_element_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.elements.len()).collect();
        indices.sort_by_key(|&i| self.elements[i].z_order);
        indices
    }

    /// Renders menu labels transformed from normalized to screen-space using canvas origin/size.
    pub(crate) fn render_label<C: BishopContext>(
        ctx: &mut C,
        label: &LabelElement,
        rect: Rect,
        display_text: &str,
    ) {
        let txt_dims = ctx.measure_text(display_text, label.font_size);
        let txt_x = match label.alignment {
            HorizontalAlign::Left => rect.x,
            HorizontalAlign::Center => rect.x + (rect.w - txt_dims.width) / 2.0,
            HorizontalAlign::Right => rect.x + rect.w - txt_dims.width,
        };
        let txt_y = rect.y + (rect.h - txt_dims.height) / 2.0 + txt_dims.offset_y;
        ctx.draw_text(display_text, txt_x, txt_y, label.font_size, label.color);
    }

    /// Returns button elements in z_order with their data.
    pub fn buttons(&self) -> Vec<(&ButtonElement, Rect, bool)> {
        let mut result = Vec::new();
        for i in self.sorted_element_indices() {
            let element = &self.elements[i];
            match &element.kind {
                MenuElementKind::Button(button) => {
                    result.push((button, element.rect, element.enabled));
                }
                MenuElementKind::LayoutGroup(group) => {
                    let resolved = resolve_layout(group, element.rect);
                    for (child, rect) in group.children.iter().zip(resolved.iter()) {
                        if let MenuElementKind::Button(button) = &child.element.kind {
                            result.push((button, *rect, child.element.enabled));
                        }
                    }
                }
                _ => {}
            }
        }
        result
    }

    /// Returns the element at the given focus position.
    pub fn get_element_at_focus(&self, focus: &MenuFocus) -> Option<&MenuElement> {
        let element = self.elements.get(focus.node)?;
        match (&element.kind, focus.child) {
            (MenuElementKind::LayoutGroup(_), Some(child_idx)) => {
                self.get_focusable_child(focus.node, child_idx)
            }
            _ => Some(element),
        }
    }

    /// Counts focusable children (buttons or sliders) in a layout group at the given element index.
    pub fn focusable_child_count(&self, element_index: usize) -> usize {
        let Some(element) = self.elements.get(element_index) else {
            return 0;
        };
        let MenuElementKind::LayoutGroup(group) = &element.kind else {
            return 0;
        };
        group
            .children
            .iter()
            .filter(|child| {
                matches!(
                    child.element.kind,
                    MenuElementKind::Button(_) | MenuElementKind::Slider(_)
                ) && child.element.enabled
                    && child.element.visible
            })
            .count()
    }

    /// Gets the nth focusable child (button or slider) in a layout group.
    pub fn get_focusable_child(
        &self,
        element_index: usize,
        child_index: usize,
    ) -> Option<&MenuElement> {
        let element = self.elements.get(element_index)?;
        let MenuElementKind::LayoutGroup(group) = &element.kind else {
            return None;
        };
        group
            .children
            .iter()
            .filter(|child| {
                matches!(
                    child.element.kind,
                    MenuElementKind::Button(_) | MenuElementKind::Slider(_)
                ) && child.element.enabled
                    && child.element.visible
            })
            .nth(child_index)
            .map(|child| &child.element)
    }
}
