use super::super::PropertyModule;
use crate::gui::widgets::script_module_core::ScriptModuleCore;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::room::Room;

/// Assigns and edits a script on the room's singleton entity.
pub struct RoomScriptModule {
    core: ScriptModuleCore,
}

impl RoomScriptModule {
    /// Creates a new room script module.
    pub fn new() -> Self {
        Self {
            core: ScriptModuleCore::new(),
        }
    }
}

impl Default for RoomScriptModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<Room> for RoomScriptModule {
    fn visible(&self, room: &Room, _game_ctx: &GameCtxMut) -> bool {
        room.singleton.is_some()
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        room: &mut Room,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let Some(entity) = room.singleton else {
            return;
        };
        self.core.draw(ctx, rect, entity, game_ctx, false);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        self.core.body_layout()
    }

    fn title(&self) -> &str {
        "Script"
    }
}
