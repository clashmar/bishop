use crate::gui::panels::panel_manager::is_mouse_over_panel;
use bishop::prelude::*;
use engine_core::prelude::*;

/// Returns true when the canvas should not receive keyboard shortcuts.
pub fn shortcuts_blocked() -> bool {
    input_is_focused() || is_modal_open() || is_context_menu_open() || is_panel_focused()
}

/// Returns true when global UI (dropdowns, modals, context menus, floating panels) is under the mouse.
pub fn canvas_blocked_by_global_ui(ctx: &WgpuContext) -> bool {
    is_dropdown_open() || is_modal_open() || is_context_menu_open() || is_mouse_over_panel(ctx)
}
