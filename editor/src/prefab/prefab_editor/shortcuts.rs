use super::PrefabEditor;
use crate::gui::modal::is_modal_open;
use bishop::prelude::*;
use engine_core::prelude::*;

impl PrefabEditor {
    pub(crate) fn handle_shortcuts(&mut self, ctx: &WgpuContext) {
        if input_is_focused() || is_modal_open() {
            return;
        }

        if Controls::r(ctx) {
            self.needs_camera_reset = true;
        }
    }
}
