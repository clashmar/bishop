use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::worlds::{InteriorZone, RoomId, WorldId};

/// Undoable room command that replaces the authored back-layer interior zones.
#[derive(Debug)]
pub struct UpdateInteriorZonesCmd {
    world_id: WorldId,
    room_id: RoomId,
    old_zones: Vec<InteriorZone>,
    new_zones: Vec<InteriorZone>,
}

impl UpdateInteriorZonesCmd {
    /// Creates a command that swaps one room's interior-zone list.
    pub fn new(
        world_id: WorldId,
        room_id: RoomId,
        old_zones: Vec<InteriorZone>,
        new_zones: Vec<InteriorZone>,
    ) -> Self {
        Self {
            world_id,
            room_id,
            old_zones,
            new_zones,
        }
    }

    fn apply(&self, zones: &[InteriorZone]) {
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
            back.interior_zones = zones.to_vec();
        });
    }
}

impl EditorCommand for UpdateInteriorZonesCmd {
    fn execute(&mut self) {
        self.apply(&self.new_zones);
    }

    fn undo(&mut self) {
        self.apply(&self.old_zones);
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Room(self.room_id)
    }
}
