// editor/src/editor/sub_editor.rs
use crate::gui::modal::is_modal_open;
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use bishop::prelude::*;
use engine_core::prelude::*;

/// Contract that all sub-editors must implement.
pub trait SubEditor {
    /// Returns the UI rects tracked by this editor for mouse hit-testing.
    fn active_rects(&self) -> &[Rect];

    /// Returns whether canvas interaction should be blocked (mouse is over UI).
    /// Editors with additional UI regions should override this.
    fn should_block_canvas(&self, ctx: &WgpuContext) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects().iter().any(|r| r.contains(mouse_screen))
            || is_dropdown_open()
            || is_modal_open()
            || is_mouse_over_panel(ctx)
    }
}
