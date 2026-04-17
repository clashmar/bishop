use super::PrefabEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

impl PrefabEditor {
    pub(crate) fn handle_shortcuts(&mut self, ctx: &WgpuContext) {
        if input_is_focused() {
            return;
        }

        if Controls::r(ctx) {
            self.needs_camera_reset = true;
        }
    }
}
