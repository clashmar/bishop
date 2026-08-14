use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::worlds::{LayerCompositionMode, RoomId, WorldId};

#[derive(Debug)]
pub struct SetBackLayerCompositionModeCmd {
    world_id: WorldId,
    room_id: RoomId,
    old_mode: LayerCompositionMode,
    new_mode: LayerCompositionMode,
}

impl SetBackLayerCompositionModeCmd {
    pub fn new(
        world_id: WorldId,
        room_id: RoomId,
        old_mode: LayerCompositionMode,
        new_mode: LayerCompositionMode,
    ) -> Self {
        Self {
            world_id,
            room_id,
            old_mode,
            new_mode,
        }
    }

    fn apply(&self, mode: LayerCompositionMode) {
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
            back.composition_mode = mode;
        });
    }
}

impl EditorCommand for SetBackLayerCompositionModeCmd {
    fn execute(&mut self) {
        self.apply(self.new_mode);
    }

    fn undo(&mut self) {
        self.apply(self.old_mode);
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Room(self.room_id)
    }
}
