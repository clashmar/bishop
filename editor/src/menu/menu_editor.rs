// editor/src/menu_editor/menu_editor.rs
use crate::app::SubEditor;
use crate::gui::modal::is_modal_open;
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use crate::menu::resize_handle::ResizeHandleState;
use crate::menu::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::HashSet;

/// Tracks an in-progress drag-to-reorder operation for managed layout children.
pub(crate) struct ReorderDragState {
    pub group_index: usize,
    pub child_index: usize,
    pub drop_target: Option<usize>,
}

/// A snap guide line to draw on the canvas.
pub(crate) enum SnapLine {
    Horizontal(f32),
    Vertical(f32),
}

/// Main menu editor state.
pub struct MenuEditor {
    pub(crate) menu_list_panel: MenuListPanel,
    pub(crate) element_palette: ElementPalette,
    pub(crate) properties_panel: MenuPropertiesPanel,
    pub templates: Vec<MenuTemplate>,
    pub current_template_index: Option<usize>,
    pub selected_element_indices: HashSet<usize>,
    pub selected_child_index: Option<usize>,
    pub pending_element_type: Option<MenuElementKind>,
    pub(crate) active_rects: Vec<Rect>,
    pub(crate) dragging_element: Option<usize>,
    pub(crate) drag_offset: Vec2,
    pub(crate) drag_start_mouse: Vec2,
    pub(crate) drag_start_rects: Vec<(usize, Vec2)>,
    pub(crate) resizing_handle: Option<ResizeHandleState>,
    pub(crate) reorder_drag: Option<ReorderDragState>,
    pub(crate) snap_lines: Vec<SnapLine>,
    pub(crate) box_select_start: Option<Vec2>,
    pub(crate) box_select_active: bool,
    pub(crate) last_norm_mouse: Option<Vec2>,
    pub(crate) view_preview: bool,
    pub(crate) drag_original_element: Option<MenuElement>,
}

impl MenuEditor {
    /// Creates a new menu editor.
    pub fn new() -> Self {
        Self {
            menu_list_panel: MenuListPanel::new(),
            element_palette: ElementPalette::new(),
            properties_panel: MenuPropertiesPanel::new(),
            templates: Vec::new(),
            current_template_index: None,
            selected_element_indices: HashSet::new(),
            selected_child_index: None,
            pending_element_type: None,
            active_rects: Vec::new(),
            dragging_element: None,
            drag_offset: Vec2::ZERO,
            drag_start_mouse: Vec2::ZERO,
            drag_start_rects: Vec::new(),
            resizing_handle: None,
            reorder_drag: None,
            snap_lines: Vec::new(),
            box_select_start: None,
            box_select_active: false,
            last_norm_mouse: None,
            view_preview: false,
            drag_original_element: None,
        }
    }

    /// Returns `Some(i)` when exactly one element is selected.
    pub fn primary_selected_index(&self) -> Option<usize> {
        if self.selected_element_indices.len() == 1 {
            self.selected_element_indices.iter().next().copied()
        } else {
            None
        }
    }

    /// Updates the menu editor and handles input.
    pub fn update(&mut self, ctx: &mut WgpuContext, camera: &Camera2D) {
        if self.view_preview {
            if Controls::v(ctx) || Controls::escape(ctx) {
                self.view_preview = false;
            }
            return;
        }

        let canvas_rect = compute_canvas_rect(ctx.screen_width(), ctx.screen_height());

        let blocked = self.should_block_canvas(ctx);

        self.update_canvas(ctx, camera, canvas_rect, blocked);

        if !input_is_focused() && Controls::v(ctx) && self.current_template_index.is_some() {
            self.view_preview = true;
            self.dragging_element = None;
            self.resizing_handle = None;
            self.reorder_drag = None;
            self.pending_element_type = None;
            self.snap_lines.clear();
            self.box_select_start = None;
            self.box_select_active = false;
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext, camera: &Camera2D) {
        self.active_rects.clear();

        ctx.set_camera(camera);
        ctx.clear_background(Color::BLACK);

        if self.view_preview {
            let preview_rect = compute_preview_rect(ctx.screen_width(), ctx.screen_height());
            self.draw_preview_canvas(ctx, camera, preview_rect);
            return;
        }

        let canvas_rect = compute_canvas_rect(ctx.screen_width(), ctx.screen_height());

        // Draw canvas under ui
        self.draw_canvas(ctx, camera, canvas_rect);

        // Draw ui after canvas
        self.draw_ui(ctx);
    }

    /// Returns a reference to the current template.
    pub fn current_template(&self) -> Option<&MenuTemplate> {
        self.current_template_index
            .and_then(|i| self.templates.get(i))
    }

    /// Returns a mutable reference to the current template.
    pub fn current_template_mut(&mut self) -> Option<&mut MenuTemplate> {
        self.current_template_index
            .and_then(|i| self.templates.get_mut(i))
    }

    /// Sets all templates and selects the first one if available.
    pub fn set_templates(&mut self, templates: Vec<MenuTemplate>) {
        self.templates = templates;
        self.current_template_index = if self.templates.is_empty() {
            None
        } else {
            Some(0)
        };
        self.selected_element_indices.clear();
        self.selected_child_index = None;
    }

    /// Selects a template by index.
    pub fn select_template(&mut self, index: usize) {
        if index < self.templates.len() {
            self.current_template_index = Some(index);
            self.selected_element_indices.clear();
            self.selected_child_index = None;
        }
    }

    /// Returns a reference to the selected element or child element when a child is selected.
    /// Returns `None` when multiple elements are selected.
    pub fn selected_element(&self) -> Option<&MenuElement> {
        let template = self.current_template()?;
        let index = self.primary_selected_index()?;
        let element = template.elements.get(index)?;
        if let Some(child_idx) = self.selected_child_index {
            if let MenuElementKind::LayoutGroup(group) = &element.kind {
                return group.children.get(child_idx).map(|c| &c.element);
            }
        }
        Some(element)
    }

    /// Returns a mutable reference to the selected element or child element when a child is selected.
    /// Returns `None` when multiple elements are selected.
    pub fn selected_element_mut(&mut self) -> Option<&mut MenuElement> {
        let index = self.primary_selected_index()?;
        let template_idx = self.current_template_index?;

        if let Some(ci) = self.selected_child_index {
            return self
                .templates
                .get_mut(template_idx)
                .and_then(|t| t.elements.get_mut(index))
                .and_then(|e| {
                    if let MenuElementKind::LayoutGroup(g) = &mut e.kind {
                        g.children.get_mut(ci).map(|c| &mut c.element)
                    } else {
                        None
                    }
                });
        }

        self.templates
            .get_mut(template_idx)?
            .elements
            .get_mut(index)
    }

    /// Snapshots the selected element, applies `mutate` to produce the new state,
    /// and pushes an `UpdateElementCmd`. The mutation is applied immediately.
    pub fn push_element_update<F>(&mut self, mutate: F)
    where
        F: FnOnce(&mut MenuElement),
    {
        let Some(template_idx) = self.current_template_index else {
            return;
        };
        let Some(element_idx) = self.primary_selected_index() else {
            return;
        };
        let child_idx = self.selected_child_index;

        let Some(old_element) = self.selected_element().cloned() else {
            return;
        };
        let mut new_element = old_element.clone();
        mutate(&mut new_element);

        // Apply immediately
        if let Some(target) = self.selected_element_mut() {
            *target = new_element.clone();
        }

        crate::editor_global::push_command(Box::new(crate::commands::menu::UpdateElementCmd::new(
            template_idx,
            element_idx,
            child_idx,
            old_element,
            new_element,
        )));
    }

    /// Applies a mutation directly to the selected element for real-time preview.
    /// Caches the original element state on the first call of a drag sequence.
    pub fn preview_element_update<F>(&mut self, mutate: F)
    where
        F: FnOnce(&mut MenuElement),
    {
        if self.drag_original_element.is_none() {
            self.drag_original_element = self.selected_element().cloned();
        }
        if let Some(target) = self.selected_element_mut() {
            mutate(target);
        }
    }

    /// Commits the previewed change as a single undo-able command using the
    /// cached original element and the current element state.
    pub fn commit_element_update(&mut self) {
        let Some(old_element) = self.drag_original_element.take() else {
            return;
        };
        let Some(template_idx) = self.current_template_index else {
            return;
        };
        let Some(element_idx) = self.primary_selected_index() else {
            return;
        };
        let child_idx = self.selected_child_index;
        let Some(new_element) = self.selected_element().cloned() else {
            return;
        };

        crate::editor_global::push_command(Box::new(crate::commands::menu::UpdateElementCmd::new(
            template_idx,
            element_idx,
            child_idx,
            old_element,
            new_element,
        )));
    }

    /// Returns true when a managed child element is currently selected.
    pub fn is_selected_child_managed(&self) -> bool {
        let Some(child_idx) = self.selected_child_index else {
            return false;
        };
        let Some(parent_idx) = self.primary_selected_index() else {
            return false;
        };
        let Some(template) = self.current_template() else {
            return false;
        };
        let Some(element) = template.elements.get(parent_idx) else {
            return false;
        };
        let MenuElementKind::LayoutGroup(group) = &element.kind else {
            return false;
        };
        group
            .children
            .get(child_idx)
            .map(|c| c.managed)
            .unwrap_or(false)
    }

    #[inline]
    pub fn register_rect(&mut self, rect: Rect) -> Rect {
        self.active_rects.push(rect);
        rect
    }

    /// Initializes the camera centered on the canvas with a 1:1 screen-space mapping.
    pub fn init_camera(ctx: &WgpuContext, camera: &mut Camera2D) {
        let sw = ctx.screen_width();
        let sh = ctx.screen_height();
        camera.target = Vec2::new(sw / 2.0, sh / 2.0);
        camera.zoom = Vec2::new(2.0 / sw, 2.0 / sh);
        camera.rotation = 0.0;
        camera.offset = Vec2::ZERO;
    }
}

impl SubEditor for MenuEditor {
    fn active_rects(&self) -> &[Rect] {
        &self.active_rects
    }

    fn should_block_canvas(&self, ctx: &WgpuContext) -> bool {
        if self.view_preview {
            return true;
        }
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects.iter().any(|r| r.contains(mouse_screen))
            || is_dropdown_open()
            || is_modal_open()
            || is_mouse_over_panel(ctx)
    }
}

impl Default for MenuEditor {
    fn default() -> Self {
        Self::new()
    }
}
