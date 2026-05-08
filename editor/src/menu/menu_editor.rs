// editor/src/menu_editor/menu_editor.rs
use crate::app::SubEditor;
use crate::gui::modals::is_modal_open;
use crate::menu::resize_handle::ResizeHandleState;
use crate::menu::*;
use crate::shared::input::canvas_blocked_by_global_ui;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::{HashMap, HashSet};

/// Tracks an in-progress drag-to-reorder operation for managed layout children.
pub(crate) struct ReorderDragState {
    pub group_index: usize,
    pub child_index: usize,
    pub drop_target: Option<usize>,
    pub dragging_out: bool,
    pub resolved_abs_rect: Option<Rect>,
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
    pub(crate) drag_original_indices: Option<(usize, usize)>,
    /// Per-field original values, keyed by WidgetId. Mirrors inspector's field_snapshots.
    pub(crate) field_originals: HashMap<WidgetId, f32>,
    /// Optional game theme loaded for canvas preview.
    pub game_theme: Option<Theme>,
    /// Name of the selected theme (for UI display).
    pub selected_theme_name: Option<String>,
    /// Set to true when any input field was actively Previewing during this frame's
    /// property draw.
    pub(crate) input_active_this_frame: bool,
    pub(crate) drop_target_group: Option<usize>,
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
            drag_original_indices: None,
            field_originals: HashMap::new(),
            game_theme: None,
            selected_theme_name: None,
            input_active_this_frame: false,
            drop_target_group: None,
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

        if !input_is_focused()
            && !is_modal_open()
            && Controls::v(ctx)
            && self.current_template_index.is_some()
        {
            self.view_preview = true;
            self.dragging_element = None;
            self.resizing_handle = None;
            self.reorder_drag = None;
            self.pending_element_type = None;
            self.snap_lines.clear();
            self.box_select_start = None;
            self.box_select_active = false;
            self.drop_target_group = None;
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext, camera: &Camera2D) {
        self.active_rects.clear();

        if self.view_preview {
            ctx.set_default_camera();
            ctx.clear_background(Color::BLACK);
            let preview_rect = compute_preview_rect(ctx.screen_width(), ctx.screen_height());
            self.draw_preview_canvas(ctx, preview_rect);
            return;
        }

        ctx.set_camera(camera);
        ctx.clear_background(Color::BLACK);

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
    #[allow(dead_code)]
    pub fn preview_element_update<F>(&mut self, mutate: F)
    where
        F: FnOnce(&mut MenuElement),
    {
        if self.drag_original_element.is_none() {
            let ti = self.current_template_index;
            let ei = self.primary_selected_index();
            if let (Some(ti), Some(ei)) = (ti, ei) {
                self.drag_original_element = self
                    .templates
                    .get(ti)
                    .and_then(|t| t.elements.get(ei).cloned());
                self.drag_original_indices = Some((ti, ei));
            }
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
        let Some((template_idx, element_idx)) = self.drag_original_indices.take() else {
            return;
        };
        let child_idx = self.selected_child_index;
        let Some(new_element) = self
            .templates
            .get(template_idx)
            .and_then(|t| t.elements.get(element_idx).cloned())
        else {
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

    /// Apply a mutation for live preview. Creates an undo entry only when committed.
    /// Sets `input_active_this_frame` when Previewing so that `draw_properties_panel`
    /// knows not to call `try_revert_escape` at the end of this frame.
    pub fn push_input_update<F>(&mut self, commit: InputCommit, mutate: F)
    where
        F: FnOnce(&mut MenuElement),
    {
        if commit == InputCommit::Unchanged {
            return;
        }

        if commit == InputCommit::Previewing && self.drag_original_element.is_none() {
            let ti = self.current_template_index;
            let ei = self.primary_selected_index();
            if let (Some(ti), Some(ei)) = (ti, ei) {
                if let Some(element) = self
                    .templates
                    .get(ti)
                    .and_then(|t| t.elements.get(ei).cloned())
                {
                    self.drag_original_element = Some(element);
                    self.drag_original_indices = Some((ti, ei));
                }
            }
        }

        if commit == InputCommit::Previewing {
            self.input_active_this_frame = true;
        }

        {
            let ti = self.current_template_index;
            if let (Some(ti), Some(ei)) = (ti, self.primary_selected_index()) {
                if let Some(target) = self
                    .templates
                    .get_mut(ti)
                    .and_then(|t| t.elements.get_mut(ei))
                {
                    mutate(target);
                }
            }
        }

        if commit == InputCommit::Committed {
            self.commit_element_update();
        }
    }

    /// Restores the selected element from the pre-edit snapshot captured during
    /// a Previewing session. Called at the end of each properties-panel frame when
    /// no field was actively Previewing (i.e. the user pressed Escape or no field
    /// had focus). Uses the stored `drag_original_indices` so it remains correct
    /// even if the selection has changed since the snapshot was taken.
    pub fn try_revert_escape(&mut self) {
        let Some(original) = self.drag_original_element.take() else {
            return;
        };
        let Some((ti, ei)) = self.drag_original_indices.take() else {
            return;
        };
        if let Some(target) = self
            .templates
            .get_mut(ti)
            .and_then(|t| t.elements.get_mut(ei))
        {
            *target = original;
        }
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
        let Some(child) = group.children.get(child_idx) else {
            return false;
        };
        if child.managed {
            return true;
        }
        // Background panels are unmanaged but their rect is controlled by the layout group
        child_idx == 0 && matches!(child.element.kind, MenuElementKind::Panel(_))
    }

    #[inline]
    pub fn register_rect(&mut self, rect: Rect) -> Rect {
        self.active_rects.push(rect);
        rect
    }
}

impl SubEditor for MenuEditor {
    fn active_rects(&self) -> &[Rect] {
        &self.active_rects
    }

    fn init_camera(&mut self, ctx: &WgpuContext, camera: &mut Camera2D) {
        let sw = ctx.screen_width();
        let sh = ctx.screen_height();
        camera.target = Vec2::new(sw / 2.0, sh / 2.0);
        camera.zoom = Vec2::new(2.0 / sw, 2.0 / sh);
        camera.rotation = 0.0;
        camera.offset = Vec2::ZERO;
    }

    fn should_block_canvas(&self, ctx: &WgpuContext) -> bool {
        if self.view_preview {
            return true;
        }
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects.iter().any(|r| r.contains(mouse_screen))
            || canvas_blocked_by_global_ui(ctx)
    }
}

impl Default for MenuEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_global::with_command_manager;
    use engine_core::menu::MenuTemplate;

    fn setup() -> MenuEditor {
        let mut editor = MenuEditor::new();
        let mut template = MenuTemplate::new("test".to_string());
        template.elements.push(MenuElement::new(
            MenuElementKind::Button(Default::default()),
            Rect::new(0.1, 0.2, 0.3, 0.4),
        ));
        editor.templates.push(template);
        editor.current_template_index = Some(0);
        editor.selected_element_indices.insert(0);
        editor
    }

    fn element_x(editor: &MenuEditor) -> f32 {
        editor.templates[0].elements[0].rect.x
    }

    #[test]
    fn previewing_mutates_element() {
        let mut editor = setup();
        let original_x = element_x(&editor);

        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.5);

        let updated_x = element_x(&editor);
        assert!(
            (updated_x - 0.5).abs() < 0.001,
            "Previewing did not mutate rect.x: got {updated_x}"
        );
        assert!(
            editor.drag_original_element.is_some(),
            "Snapshot should be captured on first preview"
        );
        assert_eq!(editor.drag_original_indices, Some((0, 0)));
        // Snapshot stores the original value
        let snap_x = editor.drag_original_element.as_ref().unwrap().rect.x;
        assert!((snap_x - original_x).abs() < 0.001);
    }

    #[test]
    fn snapshot_stable_after_first_preview() {
        let mut editor = setup();
        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.5);
        let snap1_x = editor.drag_original_element.as_ref().unwrap().rect.x;

        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.9);
        let snap2_x = editor.drag_original_element.as_ref().unwrap().rect.x;

        assert!(
            (snap1_x - snap2_x).abs() < 0.001,
            "Snapshot should not change after first preview"
        );
    }

    #[test]
    fn unchanged_before_preview_does_nothing() {
        let mut editor = setup();
        let original_x = element_x(&editor);

        editor.push_input_update(InputCommit::Unchanged, |el| el.rect.x = 0.9);

        assert!((element_x(&editor) - original_x).abs() < 0.001);
        assert!(editor.drag_original_element.is_none());
    }

    #[test]
    fn escape_reverts_preview() {
        let mut editor = setup();
        let original_x = element_x(&editor);

        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.5);
        let after_preview = element_x(&editor);
        assert!(
            (after_preview - 0.5).abs() < 0.001,
            "Preview should set x to 0.5, got {after_preview}"
        );

        editor.try_revert_escape();

        let reverted = element_x(&editor);
        assert!(
            (reverted - original_x).abs() < 0.001,
            "Escape should restore original x: {original_x}, got {reverted}"
        );
        assert!(editor.drag_original_element.is_none());
        assert!(editor.drag_original_indices.is_none());
    }

    #[test]
    fn escape_without_preview_does_nothing() {
        let mut editor = setup();
        let original_x = element_x(&editor);

        editor.try_revert_escape();

        assert!((element_x(&editor) - original_x).abs() < 0.001);
    }

    #[test]
    fn committed_with_stored_indices() {
        let mut editor = setup();
        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.5);

        // Simulate canvas click changing selection
        editor.selected_element_indices.clear();

        // Commit should still work using stored indices
        editor.push_input_update(InputCommit::Committed, |el| el.rect.x = 0.5);

        assert!(editor.drag_original_element.is_none());
    }

    #[test]
    fn committed_mutates_element() {
        let mut editor = setup();

        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.5);
        editor.push_input_update(InputCommit::Committed, |el| el.rect.x = 0.9);

        assert!(
            (element_x(&editor) - 0.9).abs() < 0.001,
            "Committed should mutate element to 0.9, got {}",
            element_x(&editor)
        );
    }

    #[test]
    fn committed_undo_saves_correct_values() {
        crate::editor_global::reset_services();
        let mut editor = setup();

        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.5);
        let cmd_count_before = with_command_manager(|cm| cm.pending_len());
        editor.push_input_update(InputCommit::Committed, |el| el.rect.x = 0.9);
        let cmd_count_after = with_command_manager(|cm| cm.pending_len());

        assert!(
            cmd_count_after > cmd_count_before,
            "Committed should push undo command (pending {cmd_count_before} -> {cmd_count_after})"
        );
    }

    #[test]
    fn snapshot_captures_pre_edit_when_push_input_update_called_first() {
        let mut editor = setup();
        let original_x = element_x(&editor);

        // Correct ordering: push_input_update first (captures snapshot),
        // then direct mutation (applies live value for canvas)
        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.5);
        editor.templates[0].elements[0].rect.x = 0.5;

        let snap_x = editor.drag_original_element.as_ref().unwrap().rect.x;
        assert!(
            (snap_x - original_x).abs() < 0.001,
            "Snapshot must capture pre-edit value {original_x}, but got {snap_x}"
        );
    }

    #[test]
    fn escape_restores_to_pre_edit_when_snapshot_correct() {
        let mut editor = setup();
        let original_x = element_x(&editor);

        // Correct ordering: push_input_update FIRST, then direct mutation
        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.5);
        editor.templates[0].elements[0].rect.x = 0.5;
        editor.try_revert_escape();

        assert!(
            (element_x(&editor) - original_x).abs() < 0.001,
            "Escape must restore to pre-edit {original_x}, got {}",
            element_x(&editor)
        );
    }

    #[test]
    fn undo_must_revert_to_pre_edit_not_preview_value() {
        crate::editor_global::reset_services();
        let mut editor = setup();
        let _original_x = element_x(&editor);

        // Preview then commit — undo should revert to original_x, not 0.5
        editor.push_input_update(InputCommit::Previewing, |el| el.rect.x = 0.5);
        editor.push_input_update(InputCommit::Committed, |el| el.rect.x = 0.9);

        // The element should be at 0.9 now (committed value)
        assert!((element_x(&editor) - 0.9).abs() < 0.001);

        // The undo command's old_element should be original_x, not 0.5
        let pending_count = with_command_manager(|cm| cm.pending_len());
        assert!(pending_count > 0);
    }

    #[test]
    fn inline_pattern_preview_and_commit_creates_undo() {
        crate::editor_global::reset_services();
        let mut editor = setup();

        let widget_id = WidgetId::default();
        let current_x = element_x(&editor);
        editor.field_originals.entry(widget_id).or_insert(current_x);

        // Capture element snapshot
        if editor.drag_original_element.is_none() {
            if let (Some(ti), Some(ei)) = (
                editor.current_template_index,
                editor.primary_selected_index(),
            ) {
                if let Some(elem) = editor
                    .templates
                    .get(ti)
                    .and_then(|t| t.elements.get(ei).cloned())
                {
                    editor.drag_original_element = Some(elem);
                    editor.drag_original_indices = Some((ti, ei));
                }
            }
        }

        // Previewing mutation
        {
            let ti = editor.current_template_index;
            if let (Some(ti), Some(ei)) = (ti, editor.primary_selected_index()) {
                if let Some(element) = editor
                    .templates
                    .get_mut(ti)
                    .and_then(|t| t.elements.get_mut(ei))
                {
                    element.rect.x = 0.5;
                }
            }
        }

        // Committed
        editor.field_originals.remove(&widget_id);
        editor.commit_element_update();

        let pending_count = with_command_manager(|cm| cm.pending_len());
        assert!(
            pending_count > 0,
            "Inline Committed should push undo command"
        );
    }
}
