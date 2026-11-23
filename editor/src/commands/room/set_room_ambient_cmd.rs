use bishop::prelude::Color;
use engine_core::worlds::{RoomId, WorldId};

use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;

/// Ambient settings authored for a room.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomAmbientSettings {
    /// Room darkness in the inclusive range 0..=1.
    pub darkness: f32,
    /// Room ambient color.
    pub color: Color,
    /// Room exposure multiplier.
    pub exposure: f32,
}

/// Undoable command for updating a room's ambient settings.
#[derive(Debug)]
pub struct SetRoomAmbientCmd {
    world_id: WorldId,
    room_id: RoomId,
    old_settings: RoomAmbientSettings,
    new_settings: RoomAmbientSettings,
}

impl SetRoomAmbientCmd {
    /// Creates a command that replaces a room's ambient settings.
    pub fn new(
        world_id: WorldId,
        room_id: RoomId,
        old_settings: RoomAmbientSettings,
        new_settings: RoomAmbientSettings,
    ) -> Self {
        Self {
            world_id,
            room_id,
            old_settings,
            new_settings,
        }
    }

    fn apply(&self, settings: RoomAmbientSettings) {
        with_editor(|editor| {
            let Some(world) = editor.game.get_world_mut(self.world_id) else {
                return;
            };
            let Some(room) = world.get_room_mut(self.room_id) else {
                return;
            };
            room.darkness = settings.darkness;
            room.ambient_color = settings.color;
            room.exposure = settings.exposure;
        });
    }
}

impl EditorCommand for SetRoomAmbientCmd {
    fn execute(&mut self) {
        self.apply(self.new_settings);
    }

    fn undo(&mut self) {
        self.apply(self.old_settings);
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Room(self.room_id)
    }
}
