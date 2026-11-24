use crate::app::EditorMode;
use crate::game::GameEditorSubmode;
use crate::gui::panels::console_panel::ConsolePanel;
use crate::gui::panels::diagnostics_panel::DiagnosticsPanel;
use crate::gui::panels::generic_panel::*;
use crate::gui::panels::hierarchy_panel::HierarchyPanel;
use crate::gui::panels::prefab_browser_panel::PrefabBrowserPanel;
use crate::gui::panels::prefab_palette_panel::PrefabPalettePanel;
use crate::gui::panels::resources_panel::ResourcesPanel;
use crate::gui::panels::world_browser_panel::WorldBrowserPanel;
use crate::with_panel_manager;
use crate::Editor;
use bishop::prelude::*;
use engine_core::storage::editor_config::{get_panel_position, set_panel_position, PanelPosition};
use std::collections::HashMap;
use ::widgets::{
    color_picker_blocks_canvas, focused_panel, is_click_consumed, is_context_menu_open,
    is_modal_open, set_focused_panel,
};

pub enum PanelMode {
    Room,
    Prefab,
    World,
    Game,
    GameWorlds,
    Menu,
}

impl PanelMode {
    fn matches(&self, mode: &EditorMode) -> bool {
        matches!(
            (self, mode),
            (PanelMode::Game, EditorMode::Game(_))
                | (PanelMode::GameWorlds, EditorMode::Game(GameEditorSubmode::Worlds))
                | (PanelMode::World, EditorMode::World(_))
                | (PanelMode::Room, EditorMode::Room(_))
                | (PanelMode::Prefab, EditorMode::Prefab(_))
                | (PanelMode::Menu, EditorMode::Menu)
        )
    }
}

pub struct PanelManager {
    /// Panels ordered by z-index (last = on top).
    panels: Vec<(PanelId, GenericPanel)>,
    panel_modes: HashMap<PanelId, Vec<PanelMode>>,
}

impl PanelManager {
    pub fn new() -> Self {
        Self {
            panels: Vec::new(),
            panel_modes: HashMap::new(),
        }
    }

    pub(crate) fn has_active_escape_interaction(&self) -> bool {
        self.panels
            .iter()
            .rev()
            .any(|(_, panel)| panel.has_active_escape_interaction())
    }

    pub(crate) fn cancel_active_escape_interaction(&mut self) {
        if let Some((_, panel)) = self
            .panels
            .iter_mut()
            .rev()
            .find(|(_, panel)| panel.has_active_escape_interaction())
        {
            panel.cancel_escape_interaction();
        }
    }

    pub fn register(&mut self, panel: GenericPanel, modes: Vec<PanelMode>) {
        let mut panel = panel;
        panel.rect = restore_panel_geometry(
            panel.rect,
            get_panel_position(panel.title),
            panel.resize_limits.is_some(),
        );

        self.panel_modes.insert(panel.title, modes);
        self.panels.push((panel.title, panel));
    }

    /// Brings the panel with the given id to the front (top of z-order).
    pub fn bring_to_front(&mut self, id: PanelId) {
        if let Some(idx) = self.panels.iter().position(|(pid, _)| *pid == id) {
            let panel = self.panels.remove(idx);
            self.panels.push(panel);
        }
    }

    pub fn update_and_draw(
        &mut self,
        ctx: &mut WgpuContext,
        editor_mode: EditorMode,
        editor: &mut Editor,
    ) {
        let mouse_screen = ctx.mouse_position().into();
        let mouse_pressed = ctx.is_mouse_button_pressed(MouseButton::Left);
        let modal_open = is_modal_open();

        // Find which panel was clicked (iterate back-to-front for z-order).
        let mut clicked_panel_id: Option<PanelId> = None;
        let color_picker_blocks_canvas = color_picker_blocks_canvas();
        let can_raise_panel = mouse_pressed
            && !is_click_consumed()
            && !color_picker_blocks_canvas
            && !is_context_menu_open()
            && !modal_open;
        if can_raise_panel {
            for (id, panel) in self.panels.iter().rev() {
                if panel.visible
                    && self.panel_modes[id].iter().any(|m| m.matches(&editor_mode))
                    && panel.rect.contains(mouse_screen)
                {
                    clicked_panel_id = Some(*id);
                    break;
                }
            }
        }

        // Bring clicked panel to front and update focus.
        if let Some(id) = clicked_panel_id {
            self.bring_to_front(id);
            set_focused_panel(Some(id));
        } else if can_raise_panel {
            set_focused_panel(None);
        }

        // Find the topmost panel containing the mouse (for blocking lower panels)
        let topmost_panel_at_mouse: Option<PanelId> = self
            .panels
            .iter()
            .rev()
            .find(|(id, panel)| {
                panel.visible
                    && self.panel_modes[id].iter().any(|m| m.matches(&editor_mode))
                    && panel.rect.contains(mouse_screen)
            })
            .map(|(id, _)| *id);

        // Draw panels in order. Block panels if the mouse is over a higher-z panel.
        for (id, panel) in self.panels.iter_mut() {
            if !self.panel_modes[id].iter().any(|m| m.matches(&editor_mode)) {
                panel.in_current_mode = false;
                continue;
            }

            panel.in_current_mode = true;

            if focused_panel() == Some(*id) && (!panel.visible || !panel.in_current_mode) {
                panel.defocus();
            }

            // Skip hidden panels
            if !panel.visible {
                continue;
            }

            // Block this panel if a modal is open or the mouse is over a different (higher-z) panel
            let blocked = modal_open
                || (topmost_panel_at_mouse.is_some() && topmost_panel_at_mouse != Some(*id));
            let was_dragging = panel.dragging;
            let was_resizing = panel.resizing;

            panel.update_and_draw(ctx, editor, blocked);

            if was_resizing && !panel.resizing {
                set_panel_position(id, panel_position_after_resize(panel.rect));
            } else if was_dragging && !panel.dragging {
                set_panel_position(
                    id,
                    panel_position_after_drag(panel.rect, get_panel_position(id)),
                );
            }
        }
    }

    pub fn toggle(&mut self, id: PanelId) {
        if let Some((_, panel)) = self.panels.iter_mut().find(|(pid, _)| *pid == id) {
            panel.visible = !panel.visible;
            if panel.visible {
                self.bring_to_front(id);
                set_focused_panel(Some(id));
            }
        }
    }

    /// Register all standard panels.
    pub fn register_all_panels(&mut self, ctx: &WgpuContext) {
        self.register_world_browser_panel(ctx);

        self.register(
            GenericPanel::new(ConsolePanel::new(), ctx),
            vec![
                PanelMode::Game,
                PanelMode::World,
                PanelMode::Room,
                PanelMode::Prefab,
                PanelMode::Menu,
            ],
        );

        self.register(
            GenericPanel::new(HierarchyPanel::new(), ctx),
            vec![PanelMode::Room, PanelMode::Prefab],
        );

        self.register(
            GenericPanel::new(DiagnosticsPanel::new(), ctx),
            vec![
                PanelMode::Game,
                PanelMode::World,
                PanelMode::Room,
                PanelMode::Prefab,
                PanelMode::Menu,
            ],
        );

        self.register(
            GenericPanel::new(PrefabPalettePanel::new(), ctx),
            vec![PanelMode::Room],
        );

        self.register(
            GenericPanel::new(ResourcesPanel::new(), ctx),
            vec![
                PanelMode::Game,
                PanelMode::World,
                PanelMode::Room,
                PanelMode::Prefab,
                PanelMode::Menu,
            ],
        );

        self.register_prefab_browser_panel(ctx);
    }

    fn register_world_browser_panel(&mut self, ctx: &WgpuContext) {
        let mut panel = GenericPanel::new(WorldBrowserPanel::new(), ctx);
        panel.visible = true;
        self.register(panel, vec![PanelMode::GameWorlds]);
    }

    fn register_prefab_browser_panel(&mut self, ctx: &WgpuContext) {
        self.register(
            GenericPanel::new(PrefabBrowserPanel::new(), ctx),
            vec![PanelMode::Prefab],
        );
    }
}

/// Returns whether a panel should block interaction.
pub fn is_mouse_over_panel(ctx: &WgpuContext) -> bool {
    with_panel_manager(|pm| {
        let mouse_screen = ctx.mouse_position().into();
        pm.panels.iter().any(|(_, p)| {
            p.visible && p.in_current_mode && (p.rect.contains(mouse_screen) || p.dragging)
        })
    })
}

fn restore_panel_geometry(
    default_rect: Rect,
    saved: Option<PanelPosition>,
    resizable: bool,
) -> Rect {
    let Some(saved) = saved else {
        return default_rect;
    };

    let mut rect = default_rect;
    rect.x = saved.x;
    rect.y = saved.y;
    if resizable {
        rect.w = saved.width.unwrap_or(default_rect.w);
        rect.h = saved.height.unwrap_or(default_rect.h);
    }
    rect
}

fn panel_position_after_drag(rect: Rect, saved: Option<PanelPosition>) -> PanelPosition {
    let (width, height) = match saved {
        Some(position) => (position.width, position.height),
        None => (None, None),
    };
    PanelPosition {
        x: rect.x,
        y: rect.y,
        width,
        height,
    }
}

fn panel_position_after_resize(rect: Rect) -> PanelPosition {
    PanelPosition {
        x: rect.x,
        y: rect.y,
        width: Some(rect.w),
        height: Some(rect.h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::worlds::*;
    use engine_core::prefab::*;
    use crate::prefab::BLANK_PREFAB_ID;

    #[test]
    fn prefab_panels_match_prefab_editor_mode() {
        assert!(PanelMode::Prefab.matches(&EditorMode::Prefab(PrefabId(3))));
        assert!(PanelMode::Prefab.matches(&EditorMode::Prefab(BLANK_PREFAB_ID)));
    }

    #[test]
    fn world_browser_panel_matches_only_game_worlds_view() {
        assert!(PanelMode::GameWorlds.matches(&EditorMode::Game(GameEditorSubmode::Worlds)));
        assert!(!PanelMode::GameWorlds.matches(&EditorMode::Game(GameEditorSubmode::Topology(WorldId(3)))));
        assert!(!PanelMode::GameWorlds.matches(&EditorMode::World(WorldId(3))));
    }

    #[test]
    fn room_panels_match_room_editor_mode() {
        assert!(PanelMode::Room.matches(&EditorMode::Room(RoomId(3))));
    }

    #[test]
    fn panel_geometry_restoration_uses_default_size_when_saved_size_is_missing() {
        let default_rect = Rect::new(10.0, 20.0, 260.0, 400.0);
        let saved = PanelPosition {
            x: 30.0,
            y: 40.0,
            width: None,
            height: None,
        };

        let restored = restore_panel_geometry(default_rect, Some(saved), true);
        assert_eq!(
            (restored.x, restored.y, restored.w, restored.h),
            (30.0, 40.0, 260.0, 400.0)
        );
    }

    #[test]
    fn panel_geometry_restoration_uses_saved_size_when_resizable() {
        let default_rect = Rect::new(10.0, 20.0, 260.0, 400.0);
        let saved = PanelPosition {
            x: 30.0,
            y: 40.0,
            width: Some(320.0),
            height: Some(480.0),
        };

        let restored = restore_panel_geometry(default_rect, Some(saved), true);
        assert_eq!(
            (restored.x, restored.y, restored.w, restored.h),
            (30.0, 40.0, 320.0, 480.0)
        );
    }

    #[test]
    fn panel_geometry_restoration_keeps_default_size_when_fixed() {
        let default_rect = Rect::new(10.0, 20.0, 260.0, 400.0);
        let saved = PanelPosition {
            x: 30.0,
            y: 40.0,
            width: Some(320.0),
            height: Some(480.0),
        };

        let restored = restore_panel_geometry(default_rect, Some(saved), false);
        assert_eq!(
            (restored.x, restored.y, restored.w, restored.h),
            (30.0, 40.0, 260.0, 400.0)
        );
    }

    #[test]
    fn panel_geometry_after_drag_preserves_saved_size() {
        let rect = Rect::new(30.0, 40.0, 260.0, 400.0);
        let saved = PanelPosition {
            x: 10.0,
            y: 20.0,
            width: Some(320.0),
            height: Some(480.0),
        };

        let geometry = panel_position_after_drag(rect, Some(saved));

        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (30.0, 40.0, Some(320.0), Some(480.0))
        );
    }

    #[test]
    fn panel_geometry_after_resize_saves_current_size() {
        let rect = Rect::new(30.0, 40.0, 320.0, 480.0);

        let geometry = panel_position_after_resize(rect);

        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (30.0, 40.0, Some(320.0), Some(480.0))
        );
    }
}
