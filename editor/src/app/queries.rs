use crate::app::{Editor, EditorMode};
use crate::game::GameEditorSubmode;
use crate::storage::game_io::list_game_names;
use engine_core::ecs::*;
use engine_core::ui::Toast;
use engine_core::worlds::*;

impl Editor {
    /// Returns the display name of the currently active game/world/entity etc (if any).
    pub fn active_editor_entity_name(&self) -> String {
        match self.mode {
            EditorMode::Game(GameEditorSubmode::Worlds) => self.game.name.clone(),
            EditorMode::Game(GameEditorSubmode::Topology(world_id)) => self
                .game
                .get_world(world_id)
                .map(|world| world.name.clone())
                .unwrap_or_else(|| self.game.name.clone()),
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

    pub(crate) fn duplicate_game_exists(&mut self, name: &str) -> bool {
        let duplicate_exists = list_game_names().iter().any(|existing| existing == name);

        if duplicate_exists {
            self.toast = Some(Toast::new(format!("\"{name}\" already exists."), 2.5));
        };

        duplicate_exists
    }

    pub(crate) fn duplicate_prefab_name_exists_excluding(
        &mut self,
        name: &str,
        exclude_id: PrefabId,
    ) -> bool {
        let duplicate_exists = self
            .game
            .prefab_manager
            .prefabs
            .iter()
            .any(|(&id, prefab)| id != exclude_id && prefab.name == name);

        if duplicate_exists {
            self.toast = Some(Toast::new(
                format!("A prefab named \"{name}\" already exists."),
                2.5,
            ));
        }

        duplicate_exists
    }
}
