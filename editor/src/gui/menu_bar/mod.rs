// editor/src/gui/menu_bar.rs
use crate::app::EditorMode;
use crate::gui::gui_constants::*;
use crate::gui::menu_widgets::menu_dropdown;
pub(crate) use crate::gui::menu_widgets::{menu_button, menu_button_text_position};
use crate::prefab::BLANK_PREFAB_ID;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::fmt;
use strum_macros::EnumIter;

/// Holds the state of the top-level menu bar.
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
    ViewPrefabBrowserPanel,
    ViewPrefabPalettePanel,
    // Options actions
    WorldSettings,
    // Editors actions
    OpenMenuEditor,
    OpenPrefabEditor,
    ReturnToGameEditor,
}

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
            EditorAction::ViewPrefabBrowserPanel => "Prefab Browser".to_string(),
            EditorAction::ViewPrefabPalettePanel => "Prefab Palette".to_string(),
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
                EditorAction::ViewDiagnosticsPanel => Some("F3"),
                EditorAction::ViewPrefabBrowserPanel => Some("P"),
                EditorAction::ViewPrefabPalettePanel => Some("P"),
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
                EditorAction::ViewPrefabBrowserPanel => Some("P"),
                EditorAction::ViewPrefabPalettePanel => Some("P"),
                _ => None,
            }
        }

        // Fallback
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    pub(crate) fn is_available_in(self, editor_mode: EditorMode) -> bool {
        match self {
            EditorAction::Rename => {
                matches!(
                    editor_mode,
                    EditorMode::Game | EditorMode::World(_) | EditorMode::Room(_)
                ) || matches!(editor_mode, EditorMode::Prefab(prefab_id) if prefab_id != BLANK_PREFAB_ID)
            }
            EditorAction::NewGame
            | EditorAction::Open
            | EditorAction::Export
            | EditorAction::ChangeSaveRoot
            | EditorAction::Undo
            | EditorAction::Redo
            | EditorAction::ViewConsolePanel
            | EditorAction::ViewDiagnosticsPanel => true,
            EditorAction::Save => !matches!(editor_mode, EditorMode::Prefab(BLANK_PREFAB_ID)),
            EditorAction::SaveAs => !matches!(editor_mode, EditorMode::Prefab(BLANK_PREFAB_ID)),
            EditorAction::ViewHierarchyPanel => {
                matches!(editor_mode, EditorMode::Room(_) | EditorMode::Prefab(_))
            }
            EditorAction::ViewPrefabBrowserPanel => matches!(editor_mode, EditorMode::Prefab(_)),
            EditorAction::ViewPrefabPalettePanel => matches!(editor_mode, EditorMode::Room(_)),
            EditorAction::WorldSettings => {
                matches!(editor_mode, EditorMode::World(_) | EditorMode::Room(_))
            }
            EditorAction::OpenMenuEditor | EditorAction::OpenPrefabEditor => {
                !matches!(editor_mode, EditorMode::Menu | EditorMode::Prefab(_))
            }
            EditorAction::ReturnToGameEditor => {
                matches!(editor_mode, EditorMode::Menu | EditorMode::Prefab(_))
            }
        }
    }

    pub(crate) fn shortcut_pressed(self, ctx: &WgpuContext) -> bool {
        match self {
            EditorAction::Save => Controls::save(ctx),
            EditorAction::SaveAs => Controls::save_as(ctx),
            EditorAction::Undo => Controls::undo(ctx),
            EditorAction::Redo => Controls::redo(ctx),
            EditorAction::ViewHierarchyPanel => Controls::h(ctx),
            EditorAction::ViewConsolePanel => Controls::c(ctx),
            EditorAction::ViewDiagnosticsPanel => Controls::f3(ctx),
            EditorAction::ViewPrefabBrowserPanel => Controls::p(ctx),
            EditorAction::ViewPrefabPalettePanel => Controls::p(ctx),
            _ => false,
        }
    }

    pub(crate) fn blocked_by_focused_input(self) -> bool {
        matches!(
            self,
            EditorAction::ViewHierarchyPanel
                | EditorAction::ViewConsolePanel
                | EditorAction::ViewDiagnosticsPanel
                | EditorAction::ViewPrefabBrowserPanel
                | EditorAction::ViewPrefabPalettePanel
        )
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
            EditorMode::Game
            | EditorMode::World(_)
            | EditorMode::Room(_)
            | EditorMode::Prefab(_) => {
                if let Some(title_actions) = title_actions_for_mode(editor_mode) {
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
                } else {
                    let txt_dims = ctx.measure_text(title, HEADER_FONT_SIZE_20);
                    let txt_x = title_rect.x + PADDING / 2.0;
                    let txt_y =
                        title_rect.y + (title_rect.h - txt_dims.height) / 2.0 + txt_dims.offset_y;
                    ctx.draw_text(title, txt_x, txt_y, HEADER_FONT_SIZE_20, Color::BLACK);
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

        let file_actions = file_actions_for_mode(editor_mode);

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

fn title_actions_for_mode(editor_mode: EditorMode) -> Option<Vec<EditorAction>> {
    if matches!(editor_mode, EditorMode::Prefab(BLANK_PREFAB_ID)) {
        None
    } else {
        Some(vec![EditorAction::Rename])
    }
}

fn file_actions_for_mode(editor_mode: EditorMode) -> Vec<EditorAction> {
    let mut actions = vec![
        EditorAction::NewGame,
        EditorAction::Open,
        EditorAction::Export,
    ];

    if !matches!(editor_mode, EditorMode::Prefab(BLANK_PREFAB_ID)) {
        actions.insert(2, EditorAction::Save);
        actions.insert(3, EditorAction::SaveAs);
    }

    if !cfg!(debug_assertions) {
        actions.push(EditorAction::ChangeSaveRoot);
    }

    actions
}

fn view_actions_for_mode(editor_mode: EditorMode) -> Vec<EditorAction> {
    [
        EditorAction::ViewConsolePanel,
        EditorAction::ViewDiagnosticsPanel,
        EditorAction::ViewHierarchyPanel,
        EditorAction::ViewPrefabBrowserPanel,
        EditorAction::ViewPrefabPalettePanel,
    ]
    .into_iter()
    .filter(|action| action.is_available_in(editor_mode))
    .collect()
}

fn options_actions_for_mode(editor_mode: EditorMode) -> Vec<EditorAction> {
    [EditorAction::WorldSettings]
        .into_iter()
        .filter(|action| action.is_available_in(editor_mode))
        .collect()
}

fn editors_actions_for_mode(editor_mode: EditorMode) -> Vec<EditorAction> {
    [
        EditorAction::OpenPrefabEditor,
        EditorAction::OpenMenuEditor,
        EditorAction::ReturnToGameEditor,
    ]
    .into_iter()
    .filter(|action| action.is_available_in(editor_mode))
    .collect()
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

#[cfg(test)]
mod tests;
