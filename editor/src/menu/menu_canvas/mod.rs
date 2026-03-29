// editor/src/menu/menu_canvas/mod.rs
mod drawing;
mod selection;

use crate::commands::menu::*;
use crate::editor_global::push_command;
use crate::menu::menu_editor::*;
use crate::menu::resize_handle::*;
use crate::menu::MenuEditor;
use crate::shared::selection::*;
use bishop::prelude::*;
use engine_core::prelude::*;

const SNAP_FRACTIONS: [f32; 7] = [0.0, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 1.0];
const SNAP_THRESHOLD: f32 = 0.02;

impl MenuEditor {
    /// Updates the menu editor and handles input.
    pub fn update_canvas(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        rect: Rect,
        blocked: bool,
    ) {
        let raw_mouse: Vec2 = ctx.mouse_position().into();
        let mouse = camera.screen_to_world(raw_mouse, ctx.screen_width(), ctx.screen_height());
        let canvas_origin = Vec2::new(rect.x, rect.y);
        let canvas_size = Vec2::new(rect.w, rect.h);
        let norm_mouse = screen_to_normalized(mouse, canvas_origin, canvas_size);
        self.last_norm_mouse = Some(norm_mouse);

        let shift_held =
            ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);

        // Arrow key movement for selected elements
        if !self.selected_element_indices.is_empty()
            && self.dragging_element.is_none()
            && !self.box_select_active
            && !input_is_focused()
        {
            let dir = get_omni_input_pressed(ctx);
            if dir != Vec2::ZERO {
                if let Some(template_idx) = self.current_template_index {
                    let step = Vec2::new(dir.x / 1920.0, dir.y / 1080.0);
                    let mut moves = Vec::new();

                    if let Some(child_idx) = self.selected_child_index {
                        if let Some(parent_idx) = self.primary_selected_index() {
                            let from = self
                                .current_template()
                                .and_then(|t| t.elements.get(parent_idx))
                                .and_then(|e| match &e.kind {
                                    MenuElementKind::LayoutGroup(g) => g.children.get(child_idx),
                                    _ => None,
                                })
                                .map(|child| Vec2::new(child.element.rect.x, child.element.rect.y));
                            if let Some(from) = from {
                                moves.push(ElementMove {
                                    element_index: parent_idx,
                                    child_index: Some(child_idx),
                                    from,
                                    to: from + step,
                                });
                            }
                        }
                    } else {
                        let indices: Vec<usize> =
                            self.selected_element_indices.iter().copied().collect();
                        if let Some(template) = self.current_template() {
                            for &i in &indices {
                                if let Some(element) = template.elements.get(i) {
                                    let from = Vec2::new(element.rect.x, element.rect.y);
                                    moves.push(ElementMove {
                                        element_index: i,
                                        child_index: None,
                                        from,
                                        to: from + step,
                                    });
                                }
                            }
                        }
                    }

                    if !moves.is_empty() {
                        push_command(Box::new(MoveElementCmd::new(template_idx, moves)));
                    }
                }
            }
        }

        // Handle Delete key to remove selected element(s)
        if !blocked && ctx.is_key_pressed(KeyCode::Delete)
            || ctx.is_key_pressed(KeyCode::Backspace)
                && !input_is_focused()
                && !self.selected_element_indices.is_empty()
        {
            if let Some(template_idx) = self.current_template_index {
                push_command(Box::new(DeleteElementCmd::new(
                    template_idx,
                    self.selected_element_indices.clone(),
                    self.selected_child_index,
                )));
            }
            return;
        }

        if blocked {
            return;
        }

        // Handle adding pending element on canvas click
        if let Some(kind) = self.pending_element_type.take() {
            if ctx.is_key_pressed(KeyCode::Escape) {
                return;
            } else if ctx.is_mouse_button_pressed(MouseButton::Left) {
                if let Some(template_idx) = self.current_template_index {
                    let default_size = match &kind {
                        MenuElementKind::Label(_) => Vec2::new(0.10, 0.03),
                        MenuElementKind::Button(_) => Vec2::new(0.10, 0.037),
                        MenuElementKind::Panel(_) => Vec2::new(0.16, 0.185),
                        MenuElementKind::LayoutGroup(_) => Vec2::new(0.25, 0.30),
                        MenuElementKind::Slider(_) => Vec2::new(0.20, 0.037),
                    };

                    // Check if a layout group is selected to add as child
                    let parent_index = self.primary_selected_index().filter(|&idx| {
                        self.current_template()
                            .and_then(|t| t.elements.get(idx))
                            .map(|e| matches!(e.kind, MenuElementKind::LayoutGroup(_)))
                            .unwrap_or(false)
                    });

                    let position = if let Some(parent_idx) = parent_index {
                        // Relative to the group origin
                        let group_origin = self
                            .current_template()
                            .and_then(|t| t.elements.get(parent_idx))
                            .map(|e| Vec2::new(e.rect.x, e.rect.y))
                            .unwrap_or(Vec2::ZERO);
                        norm_mouse - group_origin
                    } else {
                        norm_mouse
                    };

                    let rect = Rect::new(position.x, position.y, default_size.x, default_size.y);
                    let element = MenuElement::new(kind, rect);
                    push_command(Box::new(AddElementCmd::new(
                        template_idx,
                        element,
                        parent_index,
                    )));
                }
                return;
            } else {
                self.pending_element_type = Some(kind);
            }
        }

        // Handle active resize drag
        if let Some(resizing_handle) = &self.resizing_handle {
            if ctx.is_mouse_button_down(MouseButton::Left) {
                let delta = norm_mouse - resizing_handle.start_mouse;
                let original = resizing_handle.original_rect;
                let handle = resizing_handle.handle;
                let index = resizing_handle.element_index;
                let child_index = resizing_handle.child_index;
                let center_resize = ctx.is_key_down(KeyCode::LeftControl);
                let new_rect = if center_resize {
                    apply_resize_centered(original, handle, delta)
                } else {
                    apply_resize(original, handle, delta)
                };

                if let Some(child_idx) = child_index {
                    let group_origin = self
                        .current_template()
                        .and_then(|t| t.elements.get(index))
                        .map(|e| Vec2::new(e.rect.x, e.rect.y));
                    if let Some(origin) = group_origin {
                        let child = self
                            .current_template_mut()
                            .and_then(|t| t.elements.get_mut(index))
                            .and_then(|e| match &mut e.kind {
                                MenuElementKind::LayoutGroup(g) => g.children.get_mut(child_idx),
                                _ => None,
                            });
                        if let Some(child) = child {
                            child.element.rect.x = new_rect.x - origin.x;
                            child.element.rect.y = new_rect.y - origin.y;
                            child.element.rect.w = new_rect.w;
                            child.element.rect.h = new_rect.h;
                        }
                    }
                } else if let Some(template) = self.current_template_mut() {
                    if let Some(element) = template.elements.get_mut(index) {
                        element.rect = new_rect;
                    }
                }
            } else {
                // Resize ended
                if let Some(state) = self.resizing_handle.take() {
                    if let Some(template_idx) = self.current_template_index {
                        let new_rect = if let Some(child_idx) = state.child_index {
                            self.current_template()
                                .and_then(|t| t.elements.get(state.element_index))
                                .and_then(|e| match &e.kind {
                                    MenuElementKind::LayoutGroup(g) => g.children.get(child_idx),
                                    _ => None,
                                })
                                .map(|c| c.element.rect)
                        } else {
                            self.current_template()
                                .and_then(|t| t.elements.get(state.element_index))
                                .map(|e| e.rect)
                        };
                        if let Some(new_rect) = new_rect {
                            if new_rect != state.original_rect {
                                push_command(Box::new(ResizeElementCmd::new(
                                    template_idx,
                                    state.element_index,
                                    state.child_index,
                                    state.original_rect,
                                    new_rect,
                                )));
                            }
                        }
                    }
                }
            }
            return;
        }

        // Detect resize handle click on the selected element or selected child (single selection only)
        if ctx.is_mouse_button_pressed(MouseButton::Left)
            && self.try_start_resize(mouse, norm_mouse, canvas_origin, canvas_size)
        {
            return;
        }

        // Handle element selection with shift+click and box select
        if ctx.is_mouse_button_pressed(MouseButton::Left) {
            if let Some(hit) = self.hit_test_click(norm_mouse) {
                self.handle_element_click(hit, norm_mouse, shift_held);
            } else {
                // Clicked on empty space
                if !shift_held {
                    self.selected_element_indices.clear();
                    self.selected_child_index = None;
                }
                // Start box select
                self.box_select_start = Some(norm_mouse);
                self.box_select_active = true;
            }
        }

        // Handle box select drag and release
        if self.box_select_active {
            if ctx.is_mouse_button_down(MouseButton::Left) {
                return;
            } else {
                // Mouse released: finalize box select
                if let Some(start) = self.box_select_start.take() {
                    let sel_rect = rect_from_two_points(start, norm_mouse);
                    let matched: Vec<usize> = self
                        .current_template()
                        .map(|t| {
                            t.elements
                                .iter()
                                .enumerate()
                                .filter(|(_, el)| rects_intersect(sel_rect, el.rect))
                                .map(|(i, _)| i)
                                .collect()
                        })
                        .unwrap_or_default();
                    for i in matched {
                        self.selected_element_indices.insert(i);
                    }
                    self.selected_child_index = None;
                }
                self.box_select_active = false;
                return;
            }
        }

        // Handle reorder drag for managed children
        if let Some(mut reorder) = self.reorder_drag.take() {
            if ctx.is_mouse_button_down(MouseButton::Left) {
                let group_index = reorder.group_index;
                let child_index = reorder.child_index;

                let drop = self.current_template().and_then(|t| {
                    let element = t.elements.get(group_index)?;
                    if let MenuElementKind::LayoutGroup(group) = &element.kind {
                        compute_reorder_drop_index(group, element.rect, norm_mouse, child_index)
                    } else {
                        None
                    }
                });

                reorder.drop_target = drop;
                self.reorder_drag = Some(reorder);
            } else {
                let group_index = reorder.group_index;
                let child_index = reorder.child_index;
                let drop_target = reorder.drop_target;

                if let Some(target) = drop_target.filter(|&t| t != child_index) {
                    if let Some(template_idx) = self.current_template_index {
                        push_command(Box::new(ReorderChildCmd::new(
                            template_idx,
                            group_index,
                            child_index,
                            target,
                        )));
                    }
                }
            }
        }

        // Handle dragging
        if let Some(anchor_index) = self.dragging_element {
            if ctx.is_mouse_button_down(MouseButton::Left) {
                let drag_offset = self.drag_offset;
                let child_idx = self.selected_child_index;

                let norm_mouse = if shift_held {
                    let delta = norm_mouse - self.drag_start_mouse;
                    if delta.x.abs() > delta.y.abs() {
                        Vec2::new(norm_mouse.x, self.drag_start_mouse.y)
                    } else {
                        Vec2::new(self.drag_start_mouse.x, norm_mouse.y)
                    }
                } else {
                    norm_mouse
                };

                if let Some(child_idx) = child_idx {
                    let group_origin = self
                        .current_template()
                        .and_then(|t| t.elements.get(anchor_index))
                        .map(|e| Vec2::new(e.rect.x, e.rect.y));
                    if let Some(origin) = group_origin {
                        let new_abs = norm_mouse - drag_offset;
                        let child = self
                            .current_template_mut()
                            .and_then(|t| t.elements.get_mut(anchor_index))
                            .and_then(|e| match &mut e.kind {
                                MenuElementKind::LayoutGroup(g) => g.children.get_mut(child_idx),
                                _ => None,
                            });
                        if let Some(child) = child {
                            child.element.rect.x = new_abs.x - origin.x;
                            child.element.rect.y = new_abs.y - origin.y;
                        }
                    }
                } else if self.selected_element_indices.len() > 1
                    && !self.drag_start_rects.is_empty()
                {
                    let anchor_start = self
                        .drag_start_rects
                        .iter()
                        .find(|(i, _)| *i == anchor_index)
                        .map(|(_, pos)| *pos);
                    if let Some(anchor_start) = anchor_start {
                        let new_anchor_pos = norm_mouse - drag_offset;
                        let delta = new_anchor_pos - anchor_start;
                        let snapping = ctx.is_key_down(KeyCode::S) && !input_is_focused();

                        let start_rects: Vec<(usize, Vec2)> = self.drag_start_rects.clone();
                        let template_idx = self.current_template_index;
                        if let Some(ti) = template_idx {
                            let mut new_snap_lines = Vec::new();
                            let mut snap_delta = delta;
                            if let Some(template) = self.templates.get_mut(ti) {
                                if snapping {
                                    if let Some(anchor_el) = template.elements.get(anchor_index) {
                                        let anchor_size =
                                            Vec2::new(anchor_el.rect.w, anchor_el.rect.h);
                                        let (snapped, lines) = snap_center_to_fractions(
                                            anchor_start + delta,
                                            anchor_size,
                                        );
                                        snap_delta = snapped - anchor_start;
                                        new_snap_lines = lines;
                                    }
                                }
                                for &(i, start_pos) in &start_rects {
                                    if let Some(element) = template.elements.get_mut(i) {
                                        let target = start_pos + snap_delta;
                                        element.rect.x = target.x;
                                        element.rect.y = target.y;
                                    }
                                }
                            }
                            self.snap_lines = new_snap_lines;
                        }
                        if !snapping {
                            self.snap_lines.clear();
                        }
                    }
                } else {
                    // Single element drag
                    let snapping = ctx.is_key_down(KeyCode::S) && !input_is_focused();
                    if let Some(template) = self.current_template_mut() {
                        if let Some(element) = template.elements.get_mut(anchor_index) {
                            let new_pos = norm_mouse - drag_offset;
                            if snapping {
                                let size = Vec2::new(element.rect.w, element.rect.h);
                                let (snapped, lines) = snap_center_to_fractions(new_pos, size);
                                element.rect.x = snapped.x;
                                element.rect.y = snapped.y;
                                self.snap_lines = lines;
                            } else {
                                element.rect.x = new_pos.x;
                                element.rect.y = new_pos.y;
                                self.snap_lines.clear();
                            }
                        }
                    }
                }
            } else {
                if let Some(template_idx) = self.current_template_index {
                    let child_idx = self.selected_child_index;
                    let mut moves = Vec::new();

                    if let Some(template) = self.current_template() {
                        if let Some(ci) = child_idx {
                            if let Some((_, start_pos)) = self.drag_start_rects.first() {
                                let to = template
                                    .elements
                                    .get(anchor_index)
                                    .and_then(|e| match &e.kind {
                                        MenuElementKind::LayoutGroup(g) => g.children.get(ci),
                                        _ => None,
                                    })
                                    .map(|child| {
                                        Vec2::new(child.element.rect.x, child.element.rect.y)
                                    });
                                if let Some(to) = to {
                                    if to != *start_pos {
                                        moves.push(ElementMove {
                                            element_index: anchor_index,
                                            child_index: child_idx,
                                            from: *start_pos,
                                            to,
                                        });
                                    }
                                }
                            }
                        } else {
                            // Top-level drag (single or multi)
                            for &(i, start_pos) in &self.drag_start_rects {
                                if let Some(element) = template.elements.get(i) {
                                    let to = Vec2::new(element.rect.x, element.rect.y);
                                    if to != start_pos {
                                        moves.push(ElementMove {
                                            element_index: i,
                                            child_index: None,
                                            from: start_pos,
                                            to,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    if !moves.is_empty() {
                        push_command(Box::new(MoveElementCmd::new(template_idx, moves)));
                    }
                }

                self.dragging_element = None;
                self.drag_start_rects.clear();
                self.snap_lines.clear();
            }
        }
    }

    /// Checks if a resize handle was clicked and starts resizing if so.
    fn try_start_resize(
        &mut self,
        mouse: Vec2,
        norm_mouse: Vec2,
        canvas_origin: Vec2,
        canvas_size: Vec2,
    ) -> bool {
        let Some(selected_index) = self.primary_selected_index() else {
            return false;
        };

        if let Some(child_idx) = self.selected_child_index {
            let child_norm_rect = self.current_template().and_then(|t| {
                let element = t.elements.get(selected_index)?;
                let MenuElementKind::LayoutGroup(group) = &element.kind else {
                    return None;
                };
                let child = group.children.get(child_idx)?;
                if child.managed {
                    return None;
                }
                resolve_layout(group, element.rect).get(child_idx).copied()
            });
            let Some(child_norm_rect) = child_norm_rect else {
                return false;
            };
            let child_screen_rect =
                normalized_rect_to_screen(child_norm_rect, canvas_origin, canvas_size);
            let Some(handle) = hit_test_handles(mouse, child_screen_rect) else {
                return false;
            };
            self.resizing_handle = Some(ResizeHandleState {
                element_index: selected_index,
                child_index: Some(child_idx),
                handle,
                original_rect: child_norm_rect,
                start_mouse: norm_mouse,
            });
            true
        } else {
            let Some(element_rect) = self
                .current_template()
                .and_then(|t| t.elements.get(selected_index))
                .map(|e| e.rect)
            else {
                return false;
            };
            let screen_rect = normalized_rect_to_screen(element_rect, canvas_origin, canvas_size);
            let Some(handle) = hit_test_handles(mouse, screen_rect) else {
                return false;
            };
            self.resizing_handle = Some(ResizeHandleState {
                element_index: selected_index,
                child_index: None,
                handle,
                original_rect: element_rect,
                start_mouse: norm_mouse,
            });
            true
        }
    }
}

/// Computes the drop target index (in the full children Vec) from mouse position.
///
/// Compares the mouse against midpoints of managed children to determine
/// where the dragged child should be inserted.
fn compute_reorder_drop_index(
    group: &LayoutGroupElement,
    group_rect: Rect,
    norm_mouse: Vec2,
    dragged_child_index: usize,
) -> Option<usize> {
    let resolved = resolve_layout(group, group_rect);

    // Collect managed children: (vec_index, resolved_rect)
    let managed: Vec<(usize, Rect)> = group
        .children
        .iter()
        .zip(resolved.iter())
        .enumerate()
        .filter(|(_, (child, _))| child.managed)
        .map(|(idx, (_, rect))| (idx, *rect))
        .collect();

    if managed.len() < 2 {
        return None;
    }

    let managed_slot = match group.layout.direction {
        LayoutDirection::Vertical => {
            let mut slot = managed.len();
            for (i, (_, rect)) in managed.iter().enumerate() {
                let midpoint = rect.y + rect.h * 0.5;
                if norm_mouse.y < midpoint {
                    slot = i;
                    break;
                }
            }
            slot
        }
        LayoutDirection::Horizontal => {
            let mut slot = managed.len();
            for (i, (_, rect)) in managed.iter().enumerate() {
                let midpoint = rect.x + rect.w * 0.5;
                if norm_mouse.x < midpoint {
                    slot = i;
                    break;
                }
            }
            slot
        }
        LayoutDirection::Grid { columns } => {
            let cols = columns.max(1) as usize;
            if let Some((_, first_rect)) = managed.first() {
                let spacing_x = if managed.len() > 1 && cols > 1 {
                    (managed.get(1).map(|(_, r)| r.x).unwrap_or(first_rect.x)
                        - first_rect.x
                        - first_rect.w)
                        .max(0.0)
                } else {
                    0.0
                };
                let spacing_y = if managed.len() > cols {
                    (managed.get(cols).map(|(_, r)| r.y).unwrap_or(first_rect.y)
                        - first_rect.y
                        - first_rect.h)
                        .max(0.0)
                } else {
                    0.0
                };

                let cell_w = first_rect.w + spacing_x;
                let cell_h = first_rect.h + spacing_y;

                let rel_x = norm_mouse.x - first_rect.x;
                let rel_y = norm_mouse.y - first_rect.y;

                let col = (rel_x / cell_w).floor().max(0.0) as usize;
                let row = (rel_y / cell_h).floor().max(0.0) as usize;

                let col = col.min(cols - 1);
                let slot = row * cols + col;
                slot.min(managed.len())
            } else {
                0
            }
        }
    };

    // Map managed slot back to Vec index
    let dragged_managed_slot = managed
        .iter()
        .position(|(idx, _)| *idx == dragged_child_index);

    // If dropping at the same managed slot or right after, no change needed
    if let Some(d_slot) = dragged_managed_slot {
        if managed_slot == d_slot || managed_slot == d_slot + 1 {
            return Some(dragged_child_index);
        }
    }

    // Convert managed slot to Vec index
    let vec_index = if managed_slot >= managed.len() {
        // Insert after the last managed child
        managed.last().map(|(idx, _)| idx + 1).unwrap_or(0)
    } else {
        managed[managed_slot].0
    };

    Some(vec_index)
}

/// Snaps an element's center to common fractional positions on each axis.
///
/// Returns the adjusted top-left position and any active snap guide lines.
fn snap_center_to_fractions(pos: Vec2, size: Vec2) -> (Vec2, Vec<SnapLine>) {
    let center = pos + size * 0.5;
    let mut result = pos;
    let mut lines = Vec::new();

    if let Some(snapped_x) = nearest_snap(center.x) {
        result.x = snapped_x - size.x * 0.5;
        lines.push(SnapLine::Vertical(snapped_x));
    }
    if let Some(snapped_y) = nearest_snap(center.y) {
        result.y = snapped_y - size.y * 0.5;
        lines.push(SnapLine::Horizontal(snapped_y));
    }

    (result, lines)
}

/// Returns the nearest snap fraction if within threshold, or None.
fn nearest_snap(value: f32) -> Option<f32> {
    let mut best: Option<(f32, f32)> = None;
    for &frac in &SNAP_FRACTIONS {
        let dist = (value - frac).abs();
        if dist < SNAP_THRESHOLD && best.is_none_or(|(_, d)| dist < d) {
            best = Some((frac, dist));
        }
    }
    best.map(|(frac, _)| frac)
}
