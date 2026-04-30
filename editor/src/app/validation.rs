use crate::app::Editor;
use crate::storage::editor_storage::*;
use engine_core::prelude::*;

impl Editor {
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
