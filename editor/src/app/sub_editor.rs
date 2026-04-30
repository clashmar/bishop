// editor/src/editor/sub_editor.rs
use crate::shared::input::canvas_blocked_by_global_ui;
use bishop::prelude::*;

/// Contract that all sub-editors must implement.
pub trait SubEditor {
    /// Returns the UI rects tracked by this editor for mouse hit-testing.
    fn active_rects(&self) -> &[Rect];

    /// Resets the camera when entering this editor. Override for editors where
    /// the standard `ctx` + `camera` pair is sufficient; leave as a no-op for
    /// editors that require extra context (room, grid size, etc.).
    fn init_camera(&mut self, _ctx: &WgpuContext, _camera: &mut Camera2D) {}

    /// Returns whether canvas interaction should be blocked (mouse is over UI).
    /// Editors with additional UI regions should override this.
    fn should_block_canvas(&self, ctx: &WgpuContext) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects().iter().any(|r| r.contains(mouse_screen))
            || canvas_blocked_by_global_ui(ctx)
    }
}
