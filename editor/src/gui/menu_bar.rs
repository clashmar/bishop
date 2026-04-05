// editor/src/gui/menu_bar.rs
use crate::app::EditorMode;
use crate::gui::gui_constants::*;
use crate::gui::modal::is_modal_open;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Display};
use std::hash::{Hash, Hasher};
use strum_macros::EnumIter;

/// Holds the state of the top‑level menu bar.
pub struct MenuBar {
    file_id: WidgetId,
    edit_id: WidgetId,
    view_id: WidgetId,
    options_id: WidgetId,
    editors_id: WidgetId,
    title_id: WidgetId,
    pub pending: Option<EditorAction>,
}

#[derive(EnumIter, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorAction {
    // Game actions
    Rename, // Rename Game/World/Room
    // File actions
    NewGame,
    Open,
    Save,
    SaveAs,
    Export,
    ChangeSaveRoot,
    // Edit actions
    Undo,
    Redo,
    // View actions
    ViewHierarchyPanel,
    ViewConsolePanel,
    ViewDiagnosticsPanel,
    // Options actions
    WorldSettings,
    // Editors actions
    OpenMenuEditor,
    OpenPrefabEditor,
    ReturnToGameEditor,
}

const MENU_BUTTON_TARGET_SALT: u64 = 0x4D45_4E55_4254_4E31;
const MENU_ENTRY_TARGET_SALT: u64 = 0x4D45_4E55_454E_5452;

impl EditorAction {
    /// Returns the text that should be shown in dropdowns, lists, etc.
    pub fn ui_label(&self) -> String {
        match self {
            EditorAction::NewGame => "New Game".to_string(),
            EditorAction::Save => "Save".to_string(),
            EditorAction::SaveAs => "Save As".to_string(),
            EditorAction::Export => "Export".to_string(),
            EditorAction::Undo => "Undo".to_string(),
            EditorAction::Redo => "Redo".to_string(),
            EditorAction::ChangeSaveRoot => "Change Save Root".to_string(),
            EditorAction::ViewHierarchyPanel => "Hierarchy".to_string(),
            EditorAction::ViewConsolePanel => "Console".to_string(),
            EditorAction::ViewDiagnosticsPanel => "Diagnostics".to_string(),
            EditorAction::WorldSettings => "World Settings".to_string(),
            EditorAction::OpenMenuEditor => "Menu Editor".to_string(),
            EditorAction::OpenPrefabEditor => "Prefab Editor".to_string(),
            EditorAction::ReturnToGameEditor => "Game Editor".to_string(),
            _ => format!("{self:?}"),
        }
    }

    /// Optional platform-specific display string for a shortcut.
    pub fn shortcut(&self) -> Option<&'static str> {
        // Windows / Linux
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            match self {
                EditorAction::Save => Some("^ S"),
                EditorAction::SaveAs => Some("⇧ ^ S"),
                EditorAction::Undo => Some("^ Z"),
                EditorAction::Redo => Some("⇧ ^ Z"),
                EditorAction::ViewHierarchyPanel => Some("H"),
                EditorAction::ViewConsolePanel => Some("C"),
                EditorAction::ViewDiagnosticsPanel => Some("D"),
                _ => None,
            }
        }

        // macOS
        #[cfg(target_os = "macos")]
        {
            match self {
                EditorAction::Save => Some("^ S"),
                EditorAction::SaveAs => Some("⇧ ^ S"),
                EditorAction::Undo => Some("^ Z"),
                EditorAction::Redo => Some("⇧ ^ Z"),
                EditorAction::ViewHierarchyPanel => Some("H"),
                EditorAction::ViewConsolePanel => Some("C"),
                EditorAction::ViewDiagnosticsPanel => Some("F3"),
                _ => None,
            }
        }

        // Fallback
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }
}

impl fmt::Display for EditorAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ui_label())
    }
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            title_id: WidgetId::default(),
            file_id: WidgetId::default(),
            edit_id: WidgetId::default(),
            view_id: WidgetId::default(),
            options_id: WidgetId::default(),
            editors_id: WidgetId::default(),
            pending: None,
        }
    }

    /// Draw the menu options and return any requested action.
    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        title: &str,
        editor_mode: EditorMode,
    ) -> Option<EditorAction> {
        // Height of each dropdown item
        const HEIGHT: f32 = 30.0;

        // The panel is already drawn in each sub editor
        let panel_rect = menu_panel_rect(ctx);

        let mut x = panel_rect.x + PADDING;
        let y = panel_rect.y + PADDING / 2.0;

        let title_rect = Rect::new(
            x,
            y,
            rect_width_for_text(ctx, title, HEADER_FONT_SIZE_20),
            HEIGHT,
        );

        match editor_mode {
            EditorMode::Game | EditorMode::World(_) | EditorMode::Room(_) | EditorMode::Prefab(_) => {
                let title_actions = vec![EditorAction::Rename];
                if let Some(selected) = menu_dropdown(
                    ctx,
                    self.title_id,
                    title_rect,
                    title,
                    &title_actions,
                    |a| a.ui_label(),
                    |a| a.shortcut(),
                ) {
                    self.pending = Some(selected);
                }
            }
            _ => {
                let txt_dims = ctx.measure_text(title, HEADER_FONT_SIZE_20);
                let txt_x = title_rect.x + PADDING / 2.0;
                let txt_y =
                    title_rect.y + (title_rect.h - txt_dims.height) / 2.0 + txt_dims.offset_y;
                ctx.draw_text(title, txt_x, txt_y, HEADER_FONT_SIZE_20, Color::BLACK);
            }
        }

        x += title_rect.w + SPACING;

        // File dropdown
        let file_label = "File";

        let file_rect = Rect::new(
            x,
            y,
            rect_width_for_text(ctx, file_label, HEADER_FONT_SIZE_20),
            HEIGHT,
        );

        let file_actions = file_actions();

        if let Some(selected) = menu_dropdown(
            ctx,
            self.file_id,
            file_rect,
            file_label,
            &file_actions,
            |a| a.ui_label(),
            |a| a.shortcut(),
        ) {
            self.pending = Some(selected);
        }

        x += file_rect.w + SPACING;

        // Edit dropdown
        let edit_label = "Edit";

        let edit_rect = Rect::new(
            x,
            y,
            rect_width_for_text(ctx, edit_label, HEADER_FONT_SIZE_20),
            HEIGHT,
        );

        let edit_actions: Vec<EditorAction> = vec![EditorAction::Undo, EditorAction::Redo];

        if let Some(selected) = menu_dropdown(
            ctx,
            self.edit_id,
            edit_rect,
            edit_label,
            &edit_actions,
            |a| a.ui_label(),
            |a| a.shortcut(),
        ) {
            self.pending = Some(selected);
        }

        x += edit_rect.w + SPACING;

        // View dropdown
        let view_label = "View";

        let view_rect = Rect::new(
            x,
            y,
            rect_width_for_text(ctx, view_label, HEADER_FONT_SIZE_20),
            HEIGHT,
        );

        let view_actions = view_actions_for_mode(editor_mode);

        if let Some(selected) = menu_dropdown(
            ctx,
            self.view_id,
            view_rect,
            view_label,
            &view_actions,
            |a| a.ui_label(),
            |a| a.shortcut(),
        ) {
            self.pending = Some(selected);
        }

        x += view_rect.w + SPACING;

        // Options dropdown (only visible in World/Room modes)
        let options_actions = options_actions_for_mode(editor_mode);

        if !options_actions.is_empty() {
            let options_label = "Options";

            let options_rect = Rect::new(
                x,
                y,
                rect_width_for_text(ctx, options_label, HEADER_FONT_SIZE_20),
                HEIGHT,
            );

            if let Some(selected) = menu_dropdown(
                ctx,
                self.options_id,
                options_rect,
                options_label,
                &options_actions,
                |a| a.ui_label(),
                |a| a.shortcut(),
            ) {
                self.pending = Some(selected);
            }

            x += options_rect.w + SPACING;
        }

        // Editors dropdown
        let editors_label = "Editors";

        let editors_rect = Rect::new(
            x,
            y,
            rect_width_for_text(ctx, editors_label, HEADER_FONT_SIZE_20),
            HEIGHT,
        );

        let editors_actions = editors_actions_for_mode(editor_mode);

        if let Some(selected) = menu_dropdown(
            ctx,
            self.editors_id,
            editors_rect,
            editors_label,
            &editors_actions,
            |a| a.ui_label(),
            |a| a.shortcut(),
        ) {
            self.pending = Some(selected);
        }

        // Return the action
        self.pending.take()
    }
}

fn file_actions() -> Vec<EditorAction> {
    let mut actions = vec![
        EditorAction::NewGame,
        EditorAction::Open,
        EditorAction::Save,
        EditorAction::SaveAs,
        EditorAction::Export,
    ];

    if !cfg!(debug_assertions) {
        actions.push(EditorAction::ChangeSaveRoot);
    }

    actions
}

fn view_actions_for_mode(editor_mode: EditorMode) -> Vec<EditorAction> {
    let mut actions = vec![
        EditorAction::ViewConsolePanel,
        EditorAction::ViewDiagnosticsPanel,
    ];

    if matches!(editor_mode, EditorMode::Room(_) | EditorMode::Prefab(_)) {
        actions.push(EditorAction::ViewHierarchyPanel);
    }

    actions
}

fn options_actions_for_mode(editor_mode: EditorMode) -> Vec<EditorAction> {
    if matches!(editor_mode, EditorMode::World(_) | EditorMode::Room(_)) {
        return vec![EditorAction::WorldSettings];
    }

    Vec::new()
}

fn editors_actions_for_mode(editor_mode: EditorMode) -> Vec<EditorAction> {
    match editor_mode {
        EditorMode::Menu | EditorMode::Prefab(_) => vec![EditorAction::ReturnToGameEditor],
        _ => vec![EditorAction::OpenPrefabEditor, EditorAction::OpenMenuEditor],
    }
}

/// Draws a the panel background for the top menu across the whole width of the screen and returns its `Rect`.
pub fn draw_top_panel_full(ctx: &mut WgpuContext) -> Rect {
    let rect = menu_panel_rect(ctx);
    ctx.draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL_COLOR);
    rect
}

pub fn menu_panel_rect(ctx: &mut WgpuContext) -> Rect {
    Rect::new(0.0, 0.0, ctx.screen_width(), MENU_PANEL_HEIGHT)
}

/// Button and dropdown for a menu option.
fn menu_dropdown<T: Clone + PartialEq + Display>(
    ctx: &mut WgpuContext,
    id: WidgetId,
    rect: Rect,
    label: &str,
    options: &[T],
    to_string: impl Fn(&T) -> String,
    shortcut: impl Fn(&T) -> Option<&str>,
) -> Option<T> {
    const W_PADDING: f32 = 8.0;
    const DROPDOWN_Y_OFFSET: f32 = 7.5;

    // Load previous state
    let mut state = dropdown_state::get(id);

    let mouse_pos: Vec2 = ctx.mouse_position().into();
    let hovered = rect.contains(mouse_pos);

    // Change dropdown on mouse hover (if a dropdown is open)
    if hovered {
        let any_open = DROPDOWN_OPEN.with(|f| *f.borrow());
        if any_open {
            CURRENT_OPEN.with(|c| {
                let current_id = *c.borrow();
                if current_id != Some(id) {
                    // Close the previous dropdown
                    if let Some(prev_id) = current_id {
                        let mut prev_state = dropdown_state::get(prev_id);
                        prev_state.open = false;
                        dropdown_state::set(prev_id, prev_state);
                    }
                    // Open the new one
                    state.open = true;
                    *c.borrow_mut() = Some(id);
                }
            });
        }
    }

    // Dropdown header
    let button_clicked = menu_button(ctx, rect, label, state.open);

    if button_clicked {
        // Clicking the button toggles open state
        state.open = !state.open;
        // Update the global currently open dropdown
        if state.open {
            CURRENT_OPEN.with(|c| *c.borrow_mut() = Some(id));
        } else {
            CURRENT_OPEN.with(|c| *c.borrow_mut() = None);
        }
    }

    let list_is_open = state.open;

    // Let the editor know a dropdown is open
    let mut any_open = false;
    DROPDOWN_OPEN.with(|f| {
        let was = *f.borrow();
        *f.borrow_mut() = was || list_is_open;
        any_open = *f.borrow();
    });

    // Compute the widest option
    let mut max_opt_width = 0.0_f32;
    for opt in options.iter() {
        // label width
        let label_w = measure_text(ctx, &to_string(opt), DEFAULT_FONT_SIZE_16).width;
        // optional shortcut width
        let shortcut_w = shortcut(opt)
            .map(|s| measure_text(ctx, s, DEFAULT_FONT_SIZE_16).width + SPACING)
            .unwrap_or(0.0);
        let total_w = label_w + shortcut_w;
        if total_w > max_opt_width {
            max_opt_width = total_w;
        }
    }

    let list_width = rect.w.max(max_opt_width + 2.0 * W_PADDING);

    let rows = options.len();
    let total_height = rect.h * rows as f32;

    // Compute the list rectangle
    let list_rect = Rect::new(
        rect.x,
        rect.y + rect.h + DROPDOWN_Y_OFFSET,
        list_width,
        total_height,
    );

    if list_is_open {
        state.rect = list_rect;
    }

    // Draw the list and handle selection
    if list_is_open {
        let mouse_pos = ctx.mouse_position().into();

        // Background
        ctx.draw_rectangle(
            list_rect.x,
            list_rect.y,
            list_rect.w,
            list_rect.h,
            PANEL_COLOR,
        );

        for (i, opt) in options.iter().enumerate() {
            // The Y position the entry would have without scrolling
            let entry_y = list_rect.y + i as f32 * rect.h;

            let entry_rect = Rect::new(list_rect.x, entry_y, list_rect.w, rect.h);

            let hovered = entry_rect.contains(mouse_pos);
            if activate_on_release(
                MouseButton::Left,
                menu_entry_click_target(id, i),
                hovered,
                true,
                ctx.is_mouse_button_pressed(MouseButton::Left),
                ctx.is_mouse_button_released(MouseButton::Left),
            ) {
                // Close the list and return the chosen value
                state.open = false;
                dropdown_state::set(id, state);
                update_global_dropdown_flag();
                return Some(opt.clone());
            }

            if hovered {
                ctx.draw_rectangle(
                    entry_rect.x,
                    entry_rect.y,
                    entry_rect.w,
                    entry_rect.h,
                    Color::new(0.2, 0.2, 0.2, 0.9),
                );
            }

            // Action
            ctx.draw_text(
                &to_string(opt),
                entry_rect.x + 5.,
                entry_rect.y + entry_rect.h * 0.7,
                DEFAULT_FONT_SIZE_16,
                Color::BLACK,
            );

            // Optional shortcut display
            if let Some(shortcut) = shortcut(opt) {
                let sc_width = measure_text(ctx, shortcut, DEFAULT_FONT_SIZE_16).width;
                let sc_x = entry_rect.x + entry_rect.w - sc_width - 5.0;
                ctx.draw_text(
                    shortcut,
                    sc_x,
                    entry_rect.y + entry_rect.h * 0.7,
                    DEFAULT_FONT_SIZE_16,
                    Color::WHITE,
                );
            }

            // Draw the outline last
            ctx.draw_rectangle_lines(
                list_rect.x,
                list_rect.y,
                list_rect.w,
                list_rect.h,
                2.,
                Color::BLACK,
            );
        }
    }

    // Clicking outside closes the dropdown
    let mouse_pos = ctx.mouse_position().into();
    if ctx.is_mouse_button_pressed(MouseButton::Left)
        && !rect.contains(mouse_pos)
        && !(state.open && state.rect.contains(mouse_pos))
    {
        state.open = false;
        CURRENT_OPEN.with(|c| *c.borrow_mut() = None);
    }

    // Persist the state
    dropdown_state::set(id, state);
    update_global_dropdown_flag();
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::prelude::PrefabId;

    #[test]
    fn file_menu_hides_change_save_root_in_debug_builds() {
        let actions = file_actions();

        assert!(!actions.contains(&EditorAction::ChangeSaveRoot));
    }

    #[test]
    fn prefab_mode_shows_hierarchy_in_view_menu() {
        let actions = view_actions_for_mode(EditorMode::Prefab(PrefabId(7)));

        assert!(actions.contains(&EditorAction::ViewHierarchyPanel));
    }

    #[test]
    fn prefab_mode_hides_world_options_menu() {
        let actions = options_actions_for_mode(EditorMode::Prefab(PrefabId(7)));

        assert!(actions.is_empty());
    }

    #[test]
    fn prefab_mode_shows_return_game_editor_in_editors_menu() {
        let actions = editors_actions_for_mode(EditorMode::Prefab(PrefabId(7)));

        assert_eq!(actions, vec![EditorAction::ReturnToGameEditor]);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn file_menu_shows_change_save_root_in_release_builds() {
        let actions = file_actions();

        assert!(actions.contains(&EditorAction::ChangeSaveRoot));
    }
}

/// Returns true if clicked
pub fn menu_button(ctx: &mut WgpuContext, rect: Rect, label: &str, is_dropdown_open: bool) -> bool {
    // Text layout
    let txt_dims = ctx.measure_text(label, HEADER_FONT_SIZE_20);
    let (txt_x, txt_y) = menu_button_text_position(rect, txt_dims);

    let mouse = ctx.mouse_position();
    let hovered = rect.contains(vec2(mouse.0, mouse.1));

    if (hovered || is_dropdown_open)
        && !is_modal_open()
        && !ctx.is_mouse_button_down(MouseButton::Left)
    {
        ctx.draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.0, 0.0, 0.0, 0.5),
        );
    }

    ctx.draw_text(label, txt_x, txt_y, HEADER_FONT_SIZE_20, Color::BLACK);

    activate_on_release(
        MouseButton::Left,
        menu_button_click_target(rect, label),
        hovered,
        !is_modal_open(),
        ctx.is_mouse_button_pressed(MouseButton::Left),
        ctx.is_mouse_button_released(MouseButton::Left),
    )
}

pub(crate) fn menu_button_text_position(rect: Rect, txt_dims: TextDimensions) -> (f32, f32) {
    let txt_x = rect.x + (rect.w - txt_dims.width) / 2.0;
    let txt_y = rect.y + (rect.h - txt_dims.height) / 2.0 + txt_dims.offset_y - 1.0;
    (txt_x, txt_y)
}

thread_local! {
    /// Holds the `WidgetId` of the dropdown that is currently open, if any.
    static CURRENT_OPEN: RefCell<Option<WidgetId>> = const { RefCell::new(None) };
}

fn menu_button_click_target(rect: Rect, label: &str) -> ClickTargetId {
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    rect.x.to_bits().hash(&mut hasher);
    rect.y.to_bits().hash(&mut hasher);
    rect.w.to_bits().hash(&mut hasher);
    rect.h.to_bits().hash(&mut hasher);
    MENU_BUTTON_TARGET_SALT.hash(&mut hasher);
    ClickTargetId(hasher.finish())
}

fn menu_entry_click_target(id: WidgetId, index: usize) -> ClickTargetId {
    ClickTargetId(((id.0 as u64) << 32) ^ index as u64 ^ MENU_ENTRY_TARGET_SALT)
}
