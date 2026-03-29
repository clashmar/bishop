// editor/src/gui/panels/panel_manager.rs
use crate::app::EditorMode;
use crate::gui::panels::console_panel::ConsolePanel;
use crate::gui::panels::diagnostics_panel::DiagnosticsPanel;
use crate::gui::panels::generic_panel::*;
use crate::gui::panels::hierarchy_panel::HierarchyPanel;
use crate::with_panel_manager;
use crate::Editor;
use bishop::prelude::*;
use std::collections::HashMap;

pub enum PanelMode {
    Room,
    World,
    Game,
    Menu,
}

impl PanelMode {
    fn matches(&self, mode: &EditorMode) -> bool {
        matches!(
            (self, mode),
            (PanelMode::Game, EditorMode::Game)
                | (PanelMode::World, EditorMode::World(_))
                | (PanelMode::Room, EditorMode::Room(_))
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

    pub fn register(&mut self, panel: GenericPanel, modes: Vec<PanelMode>) {
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

        // Find which panel was clicked (iterate back-to-front for z-order).
        let mut clicked_panel_id: Option<PanelId> = None;
        if mouse_pressed {
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

        // Bring clicked panel to front.
        if let Some(id) = clicked_panel_id {
            self.bring_to_front(id);
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

            // Skip hidden panels
            if !panel.visible {
                continue;
            }

            // Block this panel if the mouse is over a different (higher-z) panel
            let blocked = topmost_panel_at_mouse.is_some() && topmost_panel_at_mouse != Some(*id);

            panel.update_and_draw(ctx, editor, blocked);
        }
    }

    pub fn toggle(&mut self, id: PanelId) {
        if let Some((_, panel)) = self.panels.iter_mut().find(|(pid, _)| *pid == id) {
            panel.visible = !panel.visible;
        }
    }

    /// Register all standard panels.
    pub fn register_all_panels(&mut self, ctx: &WgpuContext) {
        self.register(
            GenericPanel::new(ConsolePanel::new(), ctx),
            vec![
                PanelMode::Game,
                PanelMode::World,
                PanelMode::Room,
                PanelMode::Menu,
            ],
        );

        self.register(
            GenericPanel::new(HierarchyPanel::new(), ctx),
            vec![PanelMode::Room],
        );

        self.register(
            GenericPanel::new(DiagnosticsPanel::new(), ctx),
            vec![
                PanelMode::Game,
                PanelMode::World,
                PanelMode::Room,
                PanelMode::Menu,
            ],
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
