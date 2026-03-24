// editor/src/menu/menu_canvas/drawing.rs
use crate::shared::selection::draw_selection_box;
use crate::menu::resize_handle::*;
use crate::menu::MenuEditor;
use crate::menu::SnapLine;
use engine_core::prelude::*;
use bishop::prelude::*;

impl MenuEditor {
    /// Renders the canvas.
    pub fn draw_canvas(&self, ctx: &mut WgpuContext, camera: &Camera2D, rect: Rect) {
        ctx.draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.15, 0.15, 0.2, 1.0));

        ctx.draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, Color::new(0.4, 0.4, 0.4, 1.0));

        let canvas_origin = Vec2::new(rect.x, rect.y);
        let canvas_size = Vec2::new(rect.w, rect.h);

        // Draw "Menu Canvas" watermark if no template
        if self.current_template().is_none() {
            let center_x = rect.x + rect.w * 0.5;
            let center_y = rect.y + rect.h * 0.5;
            ctx.draw_text(
                "No menu selected",
                center_x - 55.0,
                center_y,
                14.0,
                Color::new(0.4, 0.4, 0.4, 1.0),
            );
            return;
        }

        if let Some(template) = self.current_template() {
            // Render background preview
            match template.background {
                MenuBackground::SolidColor(color) => {
                    ctx.draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
                }
                MenuBackground::Dimmed(alpha) => {
                    ctx.draw_rectangle(
                        rect.x, rect.y, rect.w, rect.h,
                        Color::new(0.0, 0.0, 0.0, alpha),
                    );
                }
                MenuBackground::None => {}
            }

            // Draw snap guide lines
            let guide_color = Color::new(1.0, 1.0, 0.4, 0.4);
            for line in &self.snap_lines {
                match line {
                    SnapLine::Vertical(nx) => {
                        let screen_x = rect.x + nx * rect.w;
                        ctx.draw_rectangle(screen_x - 0.5, rect.y, 1.0, rect.h, guide_color);
                    }
                    SnapLine::Horizontal(ny) => {
                        let screen_y = rect.y + ny * rect.h;
                        ctx.draw_rectangle(rect.x, screen_y - 0.5, rect.w, 1.0, guide_color);
                    }
                }
            }

            let raw_mouse: Vec2 = ctx.mouse_position().into();
            let world_mouse = camera.screen_to_world(raw_mouse, ctx.screen_width(), ctx.screen_height());
            let sorted = template.sorted_element_indices();
            for i in sorted {
                let element = &template.elements[i];
                let is_selected = self.selected_element_indices.contains(&i);
                let element_rect = normalized_rect_to_screen(element.rect, canvas_origin, canvas_size);
                self.draw_element(ctx, element, element_rect, canvas_origin, canvas_size, is_selected, true, world_mouse, false);
            }

            // Draw placement cursor if pending
            if self.pending_element_type.is_some() && rect.contains(world_mouse) {
                let size = 32.0;
                let half = size / 2.0;
                ctx.draw_rectangle_lines(
                    world_mouse.x - half,
                    world_mouse.y - half,
                    size,
                    size,
                    2.0,
                    Color::new(0.5, 0.8, 0.5, 0.8),
                );
            }
        }

        // Draw box selection overlay
        if self.box_select_active {
            if let (Some(start), Some(current)) = (self.box_select_start, self.last_norm_mouse) {
                let start_screen = Vec2::new(
                    canvas_origin.x + start.x * canvas_size.x,
                    canvas_origin.y + start.y * canvas_size.y,
                );
                let end_screen = Vec2::new(
                    canvas_origin.x + current.x * canvas_size.x,
                    canvas_origin.y + current.y * canvas_size.y,
                );
                draw_selection_box(ctx, start_screen, end_screen);
            }
        }
    }

    /// Renders the menu fullscreen in preview mode without editor overlays.
    pub fn draw_preview_canvas(&self, ctx: &mut WgpuContext, camera: &Camera2D, rect: Rect) {
        let canvas_origin = Vec2::new(rect.x, rect.y);
        let canvas_size = Vec2::new(rect.w, rect.h);

        let Some(template) = self.current_template() else {
            return;
        };

        match template.background {
            MenuBackground::SolidColor(color) => {
                ctx.draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
            }
            MenuBackground::Dimmed(alpha) => {
                ctx.draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.0, 0.0, 0.0, alpha));
            }
            MenuBackground::None => {}
        }

        let raw_mouse: Vec2 = ctx.mouse_position().into();
        let world_mouse = camera.screen_to_world(raw_mouse, ctx.screen_width(), ctx.screen_height());

        for i in template.sorted_element_indices() {
            let element = &template.elements[i];
            let element_rect = normalized_rect_to_screen(element.rect, canvas_origin, canvas_size);
            self.draw_element(ctx, element, element_rect, canvas_origin, canvas_size, false, false, world_mouse, true);
        }
    }

    pub(crate) fn draw_element(
        &self,
        ctx: &mut WgpuContext,
        element: &MenuElement,
        element_rect: Rect,
        canvas_origin: Vec2,
        canvas_size: Vec2,
        is_selected: bool,
        allow_resize: bool,
        world_mouse: Vec2,
        preview: bool,
    ) {
        match &element.kind {
            MenuElementKind::Button(button) => {
                let display_text = button.text_key.to_string();
                Button::new(element_rect, &display_text)
                    .font_size(button.font_size)
                    .mouse_position(world_mouse)
                    .show(ctx);

                if is_selected {
                    ctx.draw_rectangle_lines(
                        element_rect.x,
                        element_rect.y,
                        element_rect.w,
                        element_rect.h,
                        2.0,
                        Color::new(0.6, 0.8, 1.0, 1.0),
                    );
                }
            }
            MenuElementKind::LayoutGroup(group) => {
                let has_child_selected = is_selected && self.selected_child_index.is_some();

                if let Some(bg) = &group.background {
                    ctx.draw_rectangle(
                        element_rect.x,
                        element_rect.y,
                        element_rect.w,
                        element_rect.h,
                        bg.render_color(),
                    );
                }

                if !preview {
                    let outline_color = if is_selected {
                        Color::new(0.6, 0.9, 0.6, 1.0)
                    } else {
                        Color::new(0.4, 0.7, 0.4, 0.8)
                    };

                    ctx.draw_rectangle_lines(
                        element_rect.x,
                        element_rect.y,
                        element_rect.w,
                        element_rect.h,
                        if is_selected { 2.0 } else { 1.0 },
                        outline_color,
                    );

                    // Label
                    let group_label = if !element.name.is_empty() {
                        format!("[{}]", element.name)
                    } else {
                        "[Layout Group]".to_string()
                    };
                    ctx.draw_text(
                        &group_label,
                        element_rect.x + 4.0,
                        element_rect.y + 12.0,
                        10.0,
                        outline_color,
                    );
                }

                // Draw children at resolved positions
                let resolved = resolve_layout(group, element.rect);
                let reorder_info = self.reorder_drag.as_ref().filter(|r| {
                    self.selected_element_indices.contains(&r.group_index)
                });
                let dragged_child_idx = reorder_info.map(|r| r.child_index);
                let drop_target = reorder_info.and_then(|r| r.drop_target);

                for (child_idx, (child, resolved_rect)) in group.children.iter().zip(resolved.iter()).enumerate() {
                    let child_screen = normalized_rect_to_screen(*resolved_rect, canvas_origin, canvas_size);
                    let is_child_selected = is_selected && self.selected_child_index == Some(child_idx);
                    let child_allow_resize = !child.managed;

                    // Dim the dragged child at its original slot
                    if dragged_child_idx == Some(child_idx) {
                        ctx.draw_rectangle(
                            child_screen.x, child_screen.y,
                            child_screen.w, child_screen.h,
                            Color::new(0.0, 0.0, 0.0, 0.3),
                        );
                    }

                    self.draw_element(ctx, &child.element, child_screen, canvas_origin, canvas_size, is_child_selected, child_allow_resize, world_mouse, preview);
                }

                // Draw drop indicator line
                if let Some(target) = drop_target {
                    let managed_rects: Vec<(usize, Rect)> = group.children.iter()
                        .zip(resolved.iter())
                        .enumerate()
                        .filter(|(_, (child, _))| child.managed)
                        .map(|(idx, (_, rect))| (idx, *rect))
                        .collect();

                    let managed_slot = child_index_to_managed_slot(group, target);

                    draw_reorder_indicator(
                        ctx, &managed_rects, managed_slot,
                        &group.layout,
                        canvas_origin, canvas_size,
                    );
                }

                // Draw resize handles on group only when no child is selected
                if is_selected && !has_child_selected {
                    draw_resize_handles(ctx, element_rect);
                }
                return;
            }
            MenuElementKind::Label(label) => {
                if !preview {
                    let outline_color = if is_selected {
                        Color::new(0.6, 0.8, 1.0, 1.0)
                    } else {
                        Color::new(0.5, 0.5, 0.5, 1.0)
                    };

                    ctx.draw_rectangle_lines(
                        element_rect.x,
                        element_rect.y,
                        element_rect.w,
                        element_rect.h,
                        if is_selected { 2.0 } else { 1.0 },
                        outline_color,
                    );
                }

                let text = &label.text_key;
                let text_dims = ctx.measure_text(text, label.font_size);
                let text_x = match label.alignment {
                    HorizontalAlign::Left => element_rect.x,
                    HorizontalAlign::Center => element_rect.x + (element_rect.w - text_dims.width) / 2.0,
                    HorizontalAlign::Right => element_rect.x + element_rect.w - text_dims.width,
                };
                let text_y = element_rect.y + (element_rect.h - text_dims.height) / 2.0 + text_dims.offset_y;
                ctx.draw_text(text, text_x, text_y, label.font_size, label.color);
            }
            MenuElementKind::Slider(slider) => {
                // Draw label area (left 40%) and track area (right 60%)
                let split = element_rect.w * 0.4;
                let track_rect = Rect::new(element_rect.x + split, element_rect.y, element_rect.w - split, element_rect.h);
                let track_h = element_rect.h * 0.2;
                let track_y = element_rect.y + (element_rect.h - track_h) * 0.5;

                ctx.draw_rectangle(track_rect.x, track_rect.y, track_rect.w, track_rect.h, Color::new(0.15, 0.15, 0.18, 1.0));
                ctx.draw_rectangle(track_rect.x, track_y, track_rect.w, track_h, Color::new(0.2, 0.2, 0.2, 0.8));

                let text_dims = ctx.measure_text(&slider.text_key, 14.0);
                let text_y = element_rect.y + (element_rect.h - text_dims.height) * 0.5 + text_dims.offset_y;
                ctx.draw_text(&slider.text_key, element_rect.x + 4.0, text_y, 14.0, Color::WHITE);

                if !preview {
                    let outline_color = if is_selected {
                        Color::new(0.6, 0.8, 1.0, 1.0)
                    } else {
                        Color::new(0.5, 0.5, 0.5, 1.0)
                    };
                    ctx.draw_rectangle_lines(element_rect.x, element_rect.y, element_rect.w, element_rect.h, if is_selected { 2.0 } else { 1.0 }, outline_color);
                }
            }
            MenuElementKind::Panel(panel) => {
                ctx.draw_rectangle(
                    element_rect.x,
                    element_rect.y,
                    element_rect.w,
                    element_rect.h,
                    panel.background.render_color(),
                );

                if !preview {
                    let outline_color = if is_selected {
                        Color::new(0.6, 0.8, 1.0, 1.0)
                    } else {
                        Color::new(0.5, 0.5, 0.5, 1.0)
                    };

                    ctx.draw_rectangle_lines(
                        element_rect.x,
                        element_rect.y,
                        element_rect.w,
                        element_rect.h,
                        if is_selected { 2.0 } else { 1.0 },
                        outline_color,
                    );

                    let label = if !element.name.is_empty() {
                        element.name.as_str()
                    } else {
                        "[Panel]"
                    };

                    ctx.draw_text(
                        label,
                        element_rect.x + 4.0,
                        element_rect.y + 12.0,
                        10.0,
                        outline_color,
                    );
                }
            }
        }

        if is_selected && allow_resize {
            draw_resize_handles(ctx, element_rect);
        }
    }
}

/// Draws a drop indicator line at the target managed slot position.
pub(crate) fn draw_reorder_indicator(
    ctx: &mut WgpuContext,
    managed_rects: &[(usize, Rect)],
    managed_slot: usize,
    layout: &LayoutConfig,
    canvas_origin: Vec2,
    canvas_size: Vec2,
) {
    if managed_rects.is_empty() {
        return;
    }

    let indicator_color = Color::new(0.3, 0.7, 1.0, 0.9);
    let thickness = 2.0;
    let spacing_x = layout.spacing / 1920.0;
    let spacing_y = layout.spacing / 1080.0;
    let direction = layout.direction;

    match direction {
        LayoutDirection::Vertical => {
            let (y, x, w) = if managed_slot == 0 {
                let (_, first) = &managed_rects[0];
                (first.y - spacing_y * 0.5, first.x, first.w)
            } else if managed_slot >= managed_rects.len() {
                let (_, last) = managed_rects.last().unwrap();
                (last.y + last.h + spacing_y * 0.5, last.x, last.w)
            } else {
                let (_, prev) = &managed_rects[managed_slot - 1];
                let (_, next) = &managed_rects[managed_slot];
                let mid_y = (prev.y + prev.h + next.y) * 0.5;
                (mid_y, next.x, next.w)
            };
            let screen = normalized_rect_to_screen(
                Rect::new(x, y - 0.001, w, 0.002),
                canvas_origin, canvas_size,
            );
            ctx.draw_rectangle(screen.x, screen.y, screen.w, thickness, indicator_color);
        }
        LayoutDirection::Horizontal => {
            let (x, y, h) = if managed_slot == 0 {
                let (_, first) = &managed_rects[0];
                (first.x - spacing_x * 0.5, first.y, first.h)
            } else if managed_slot >= managed_rects.len() {
                let (_, last) = managed_rects.last().unwrap();
                (last.x + last.w + spacing_x * 0.5, last.y, last.h)
            } else {
                let (_, prev) = &managed_rects[managed_slot - 1];
                let (_, next) = &managed_rects[managed_slot];
                let mid_x = (prev.x + prev.w + next.x) * 0.5;
                (mid_x, next.y, next.h)
            };
            let screen = normalized_rect_to_screen(
                Rect::new(x - 0.001, y, 0.002, h),
                canvas_origin, canvas_size,
            );
            ctx.draw_rectangle(screen.x, screen.y, thickness, screen.h, indicator_color);
        }
        LayoutDirection::Grid { .. } => {
            let (y, x, w) = if managed_slot == 0 {
                let (_, first) = &managed_rects[0];
                (first.y - spacing_y * 0.5, first.x, first.w)
            } else if managed_slot >= managed_rects.len() {
                let (_, last) = managed_rects.last().unwrap();
                (last.y + last.h + spacing_y * 0.5, last.x, last.w)
            } else {
                let (_, prev) = &managed_rects[managed_slot - 1];
                let (_, next) = &managed_rects[managed_slot];
                let mid_y = (prev.y + prev.h + next.y) * 0.5;
                (mid_y, next.x, next.w)
            };
            let screen = normalized_rect_to_screen(
                Rect::new(x, y - 0.001, w, 0.002),
                canvas_origin, canvas_size,
            );
            ctx.draw_rectangle(screen.x, screen.y, screen.w, thickness, indicator_color);
        }
    }
}

/// Maps a Vec child index to its managed slot index.
fn child_index_to_managed_slot(group: &LayoutGroupElement, child_index: usize) -> usize {
    group.children.iter()
        .take(child_index)
        .filter(|c| c.managed)
        .count()
}