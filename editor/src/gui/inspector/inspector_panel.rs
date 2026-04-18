// editor/src/gui/inspector/inspector_panel.rs
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use crate::shared::scene_ui::inspector::{
    SceneInspector, SceneInspectorContext, SceneInspectorOutput,
};
use bishop::prelude::*;
use engine_core::prelude::*;

/// The panel that lives on the right‑hand side of the room editor.
pub struct InspectorPanel {
    /// Geometry of the panel.
    rect: Rect,
    /// Currently inspected entity.
    pub target: Option<Entity>,
    /// Rectangles that were drawn this frame and are therefore active.
    active_rects: Vec<Rect>,
    /// Shared generic inspector core.
    core: SceneInspector,
}

impl InspectorPanel {
    /// Create a fresh panel with the default set of modules.
    pub fn new() -> Self {
        Self {
            rect: Rect::new(0., 0., 0., 0.),
            target: None,
            active_rects: Vec::new(),
            core: SceneInspector::new(),
        }
    }

    /// Called by the editor each frame to place the panel.
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    /// Tell the inspector which entity is currently selected.
    pub fn set_target(&mut self, entity: Option<Entity>) {
        self.core.set_target(entity);
        self.target = self.core.target();
    }

    /// Render the panel and any visible sub‑modules.
    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        scene_ctx: &SceneInspectorContext,
    ) -> SceneInspectorOutput {
        let draw = self.core.draw(
            ctx,
            self.rect,
            is_mouse_over_panel(ctx),
            game_ctx,
            scene_ctx,
        );
        self.active_rects = draw.interactive_rects;
        self.target = self.core.target();
        draw.output
    }

    pub fn is_mouse_over(&self, ctx: &WgpuContext) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects.iter().any(|r| r.contains(mouse_screen))
            || (self.rect.contains(mouse_screen) && self.target.is_some())
    }
}
