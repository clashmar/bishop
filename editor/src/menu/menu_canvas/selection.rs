// editor/src/menu/menu_canvas/selection.rs
use crate::menu::menu_editor::*;
use crate::menu::MenuEditor;
use engine_core::prelude::*;

pub struct HitTestResult {
    pub element_index: usize,
    pub element_rect: Rect,
    pub hit: HitKind,
}

pub enum HitKind {
    Element,
    Child {
        child_index: usize,
        rect: Rect,
        is_managed: bool,
    },
}

impl MenuEditor {
    /// Hit-tests elements at a normalized mouse position.
    pub(crate) fn hit_test_click(&self, norm_mouse: Vec2) -> Option<HitTestResult> {
        let template = self.current_template()?;
        let sorted = template.sorted_element_indices();
        for &i in sorted.iter().rev() {
            let element = &template.elements[i];
            if let MenuElementKind::LayoutGroup(group) = &element.kind {
                let resolved = resolve_layout(group, element.rect);
                for (child_idx, resolved_rect) in resolved.iter().enumerate().rev() {
                    if resolved_rect.contains(norm_mouse) {
                        let is_managed = group
                            .children
                            .get(child_idx)
                            .map(|c| c.managed)
                            .unwrap_or(true);
                        return Some(HitTestResult {
                            element_index: i,
                            element_rect: element.rect,
                            hit: HitKind::Child {
                                child_index: child_idx,
                                rect: *resolved_rect,
                                is_managed,
                            },
                        });
                    }
                }
                if element.rect.contains(norm_mouse) {
                    return Some(HitTestResult {
                        element_index: i,
                        element_rect: element.rect,
                        hit: HitKind::Element,
                    });
                }
                continue;
            }
            if element.rect.contains(norm_mouse) {
                return Some(HitTestResult {
                    element_index: i,
                    element_rect: element.rect,
                    hit: HitKind::Element,
                });
            }
        }
        None
    }

    /// Handles a click on a canvas element, updating selection and drag state.
    pub(crate) fn handle_element_click(
        &mut self,
        hit: HitTestResult,
        norm_mouse: Vec2,
        shift_held: bool,
    ) {
        let idx = hit.element_index;
        let element_rect = hit.element_rect;

        match hit.hit {
            HitKind::Child {
                child_index,
                rect: child_rect,
                is_managed,
            } => {
                self.selected_element_indices.clear();
                self.selected_element_indices.insert(idx);
                self.selected_child_index = Some(child_index);

                if is_managed {
                    self.reorder_drag = Some(ReorderDragState {
                        group_index: idx,
                        child_index,
                        drop_target: None,
                    });
                } else {
                    self.dragging_element = Some(idx);
                    self.drag_offset = norm_mouse - Vec2::new(child_rect.x, child_rect.y);
                    self.drag_start_mouse = norm_mouse;

                    let start = self
                        .current_template()
                        .and_then(|t| t.elements.get(idx))
                        .and_then(|e| match &e.kind {
                            MenuElementKind::LayoutGroup(g) => g.children.get(child_index),
                            _ => None,
                        })
                        .map(|child| Vec2::new(child.element.rect.x, child.element.rect.y));

                    if let Some(pos) = start {
                        self.drag_start_rects = vec![(idx, pos)];
                    }
                }
            }

            HitKind::Element => {
                if shift_held {
                    self.selected_child_index = None;

                    if self.selected_element_indices.contains(&idx) {
                        self.selected_element_indices.remove(&idx);
                    } else {
                        self.selected_element_indices.insert(idx);
                    }

                    return;
                }

                if self.selected_element_indices.contains(&idx) {
                    self.selected_child_index = None;
                    self.dragging_element = Some(idx);
                    self.drag_offset = norm_mouse - Vec2::new(element_rect.x, element_rect.y);
                    self.drag_start_mouse = norm_mouse;
                    let indices: Vec<usize> =
                        self.selected_element_indices.iter().copied().collect();
                    let start_rects: Vec<(usize, Vec2)> = self
                        .current_template()
                        .map(|t| {
                            indices
                                .into_iter()
                                .filter_map(|si| {
                                    t.elements
                                        .get(si)
                                        .map(|el| (si, Vec2::new(el.rect.x, el.rect.y)))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    self.drag_start_rects = start_rects;
                } else {
                    self.selected_element_indices.clear();
                    self.selected_element_indices.insert(idx);
                    self.selected_child_index = None;
                    self.dragging_element = Some(idx);
                    self.drag_offset = norm_mouse - Vec2::new(element_rect.x, element_rect.y);
                    self.drag_start_mouse = norm_mouse;
                    self.drag_start_rects = vec![(idx, Vec2::new(element_rect.x, element_rect.y))];
                }
            }
        }
    }
}
