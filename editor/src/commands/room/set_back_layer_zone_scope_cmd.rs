use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::worlds::{InteriorZoneScope, RoomId, WorldId};

#[derive(Debug)]
pub struct SetBackLayerZoneScopeCmd {
    world_id: WorldId,
    room_id: RoomId,
    old_scope: InteriorZoneScope,
    new_scope: InteriorZoneScope,
}

impl SetBackLayerZoneScopeCmd {
    pub fn new(
        world_id: WorldId,
        room_id: RoomId,
        old_scope: InteriorZoneScope,
        new_scope: InteriorZoneScope,
    ) -> Self {
        Self {
            world_id,
            room_id,
            old_scope,
            new_scope,
        }
    }

    fn apply(&self, scope: InteriorZoneScope) {
        with_editor(|editor| {
            let Some(world) = editor.game.get_world_mut(self.world_id) else {
                return;
            };
            let Some(room) = world.get_room_mut(self.room_id) else {
                return;
            };
            let Some(back) = room.current_variant_mut().layers.back.as_mut() else {
                return;
            };
            back.zone_scope = scope;
        });
    }
}

impl EditorCommand for SetBackLayerZoneScopeCmd {
    fn execute(&mut self) {
        self.apply(self.new_scope);
    }

    fn undo(&mut self) {
        self.apply(self.old_scope);
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Room(self.room_id)
    }
}
