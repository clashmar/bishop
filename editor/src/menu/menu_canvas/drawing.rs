// editor/src/menu/menu_canvas/drawing.rs
use crate::menu::resize_handle::*;
use crate::menu::MenuEditor;
use crate::menu::SnapLine;
use crate::shared::selection::draw_selection_box;
use bishop::prelude::*;
use engine_core::constants::world;
use engine_core::prelude::*;
use std::collections::HashMap;

pub(crate) struct MenuCanvasFrame<'a> {
    pub(crate) ctx: &'a mut WgpuContext,
    pub(crate) canvas_origin: Vec2,
    pub(crate) canvas_size: Vec2,
    pub(crate) world_mouse: Vec2,
    pub(crate) preview: bool,
}

impl MenuEditor {
    /// Renders the canvas.
    pub fn draw_canvas(&self, ctx: &mut WgpuContext, camera: &Camera2D, rect: Rect) {
        ctx.draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            with_theme(|theme| theme.background),
        );

        ctx.draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            2.0,
            with_theme(|theme| theme.border),
        );

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
                with_theme(|theme| theme.text),
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
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h,
                        Color::new(0.0, 0.0, 0.0, alpha),
                    );
                }
                MenuBackground::None => {}
            }

            // Draw snap guide lines
            let guide_color = with_theme(|theme| theme.highlight);
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
            let world_mouse =
                camera.screen_to_world(raw_mouse, ctx.screen_width(), ctx.screen_height());

            let mut frame = MenuCanvasFrame {
                ctx,
                canvas_origin,
                canvas_size,
                world_mouse,
                preview: false,
            };

            let editor_theme = get_theme();
            if let Some(ref game_theme) = self.game_theme {
                set_theme(game_theme.clone());
            }

            let sorted = template.sorted_element_indices();
            for i in sorted {
                let element = &template.elements[i];
                if !element.visible {
                    continue;
                }
                let is_selected = self.selected_element_indices.contains(&i);
                let element_rect =
                    normalized_rect_to_screen(element.rect, canvas_origin, canvas_size);
                self.draw_element(&mut frame, element, element_rect, is_selected, true);

                // Highlight drop-target group during drag-in hover
                if self.drop_target_group == Some(i) {
                    frame.ctx.draw_rectangle_lines(
                        element_rect.x - 2.0,
                        element_rect.y - 2.0,
                        element_rect.w + 4.0,
                        element_rect.h + 4.0,
                        3.0,
                        Color::new(0.3, 0.7, 1.0, 0.8),
                    );
                }
            }

            if self.game_theme.is_some() {
                set_theme(editor_theme);
            }

            self.draw_child_drag_overlay(&mut frame, template);

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
                    with_theme(|theme| theme.primary),
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
                draw_selection_box(ctx, start_screen, end_screen, world::DEFAULT_GRID_SIZE);
            }
        }
    }

    /// Renders the menu fullscreen in preview mode using the runtime's element rendering.
    pub fn draw_preview_canvas(&self, ctx: &mut WgpuContext, rect: Rect) {
        let Some(template) = self.current_template() else {
            return;
        };
        let focus = MenuFocus {
            node: usize::MAX,
            child: None,
        };
        let mut slider_values = HashMap::new();
        let text_manager = TextManager::default();

        let editor_theme = get_theme();
        if let Some(ref game_theme) = self.game_theme {
            set_theme(game_theme.clone());
        }

        render_menu_elements(
            ctx,
            template,
            &template.id,
            rect,
            &focus,
            &mut slider_values,
            &text_manager,
        );

        if self.game_theme.is_some() {
            set_theme(editor_theme);
        }
    }

    pub(crate) fn draw_element(
        &self,
        frame: &mut MenuCanvasFrame<'_>,
        element: &MenuElement,
        element_rect: Rect,
        is_selected: bool,
        allow_resize: bool,
    ) {
        match &element.kind {
            MenuElementKind::Button(_) => {
                self.draw_button(frame, element, element_rect, is_selected);
            }
            MenuElementKind::LayoutGroup(_) => {
                self.draw_layout_group(frame, element, element_rect, is_selected);
                return;
            }
            MenuElementKind::Label(_) => {
                self.draw_label(frame, element, element_rect, is_selected);
            }
            MenuElementKind::Slider(_) => {
                self.draw_slider(frame, element, element_rect, is_selected);
            }
            MenuElementKind::Panel(_) => {
                self.draw_panel(frame, element, element_rect, is_selected);
            }
        }

        if is_selected && allow_resize {
            draw_resize_handles(frame.ctx, element_rect);
        }
    }

    /// Returns (group, child) for the given template indices — the core lookup.
    pub(crate) fn group_and_child(
        template: &MenuTemplate,
        group_index: usize,
        child_index: usize,
    ) -> Option<(&LayoutGroupElement, &LayoutChild)> {
        let el = template.elements.get(group_index)?;
        let group = match &el.kind {
            MenuElementKind::LayoutGroup(g) => g,
            _ => return None,
        };
        let child = group.children.get(child_index)?;
        Some((group, child))
    }

    /// Mutable version for drag/update operations. Returns the child element.
    pub(crate) fn child_mut(
        template: &mut MenuTemplate,
        group_index: usize,
        child_index: usize,
    ) -> Option<&mut LayoutChild> {
        let el = template.elements.get_mut(group_index)?;
        let group = match &mut el.kind {
            MenuElementKind::LayoutGroup(g) => g,
            _ => return None,
        };
        group.children.get_mut(child_index)
    }

    /// Resolves (group, child, resolved_normalized_rect) for the given group/child indices.
    pub(crate) fn resolve_group_child(
        template: &MenuTemplate,
        group_index: usize,
        child_index: usize,
    ) -> Option<(&LayoutGroupElement, &LayoutChild, Rect)> {
        let (group, child) = Self::group_and_child(template, group_index, child_index)?;
        let resolved_rect = *resolve_layout(group, template.elements.get(group_index)?.rect)
            .get(child_index)?;
        Some((group, child, resolved_rect))
    }

    fn is_mouse_outside_element(&self, template: &MenuTemplate, element_index: usize) -> bool {
        self.last_norm_mouse
            .and_then(|nm| {
                template
                    .elements
                    .get(element_index)
                    .map(|el| !el.rect.contains(nm))
            })
            .unwrap_or(false)
    }

    fn draw_child_drag_overlay(
        &self,
        frame: &mut MenuCanvasFrame<'_>,
        template: &MenuTemplate,
    ) {
        self.draw_managed_child_ghost(frame, template);
        self.draw_unmanaged_child_outline(frame, template);
    }

    fn draw_managed_child_ghost(&self, frame: &mut MenuCanvasFrame<'_>, template: &MenuTemplate) {
        let Some(reorder) = self.reorder_drag.as_ref() else {
            return;
        };
        let Some((_group, _child, resolved_rect)) =
            Self::resolve_group_child(template, reorder.group_index, reorder.child_index)
        else {
            return;
        };

        let raw_mouse: Vec2 = frame.ctx.mouse_position().into();
        let ghost_rect = Rect::new(
            raw_mouse.x - resolved_rect.w * frame.canvas_size.x / 2.0,
            raw_mouse.y - resolved_rect.h * frame.canvas_size.y / 2.0,
            resolved_rect.w * frame.canvas_size.x,
            resolved_rect.h * frame.canvas_size.y,
        );
        self.draw_element(frame, &_child.element, ghost_rect, false, false);
        frame.ctx.draw_rectangle(
            ghost_rect.x,
            ghost_rect.y,
            ghost_rect.w,
            ghost_rect.h,
            Color::new(1.0, 1.0, 1.0, 0.3),
        );
        if reorder.dragging_out {
            draw_exit_outline(frame.ctx, ghost_rect);
        }
    }

    /// Draws an exit outline for an unmanaged child being dragged outside its parent group.
    fn draw_unmanaged_child_outline(
        &self,
        frame: &mut MenuCanvasFrame<'_>,
        template: &MenuTemplate,
    ) {
        let Some(anchor_index) = self.dragging_element else {
            return;
        };
        let Some(child_idx) = self.selected_child_index else {
            return;
        };
        if !self.is_mouse_outside_element(template, anchor_index) {
            return;
        };

        let Some((group, child, resolved_rect)) =
            Self::resolve_group_child(template, anchor_index, child_idx)
        else {
            return;
        };
        if child.managed || is_background_panel(group, child_idx) {
            return;
        };

        draw_exit_outline(
            frame.ctx,
            normalized_rect_to_screen(resolved_rect, frame.canvas_origin, frame.canvas_size),
        );
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
                canvas_origin,
                canvas_size,
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
                canvas_origin,
                canvas_size,
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
                canvas_origin,
                canvas_size,
            );
            ctx.draw_rectangle(screen.x, screen.y, screen.w, thickness, indicator_color);
        }
    }
}

fn draw_exit_outline(ctx: &mut WgpuContext, rect: Rect) {
    ctx.draw_rectangle_lines(
        rect.x - 2.0,
        rect.y - 2.0,
        rect.w + 4.0,
        rect.h + 4.0,
        3.0,
        Color::new(0.3, 0.7, 1.0, 0.8),
    );
}
