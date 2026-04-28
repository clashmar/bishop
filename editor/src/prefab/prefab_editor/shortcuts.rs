use super::PrefabEditor;
use crate::shared::input::shortcuts_blocked;
use bishop::prelude::*;
use engine_core::prelude::*;

impl PrefabEditor {
    pub(crate) fn handle_shortcuts(&mut self, ctx: &WgpuContext) {
        if shortcuts_blocked() {
            return;
        }

        if Controls::r(ctx) {
            self.needs_camera_reset = true;
        }
    }
}
