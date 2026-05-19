use crate::app::{Editor, EditorMode};
use engine_core::prelude::*;

impl Editor {
    /// Returns the display name of the currently active entity (game, world, room, or prefab).
    /// Returns `"Menu Editor"` when in menu mode.
    pub fn active_entity_name(&self) -> String {
        match self.mode {
            EditorMode::Game => self.game.name.clone(),
            EditorMode::World(_) => self.game.current_world().name.clone(),
            EditorMode::Room(id) => self
                .game
                .current_world()
                .get_room(id)
                .map(|room| room.name.clone())
                .unwrap_or_else(|| "Room".to_string()),
            EditorMode::Prefab(_) => self
                .prefab_editor
                .as_ref()
                .map(|editor| editor.prefab_name.clone())
                .unwrap_or_else(|| "Prefab".to_string()),
            EditorMode::Menu => "Menu Editor".to_string(),
        }
    }

    pub fn get_room_from_id(&self, room_id: &RoomId) -> &Room {
        self.game
            .current_world()
            .get_room(*room_id)
            .expect("Could not find room from id.")
    }
}
