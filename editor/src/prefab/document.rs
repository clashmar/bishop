use crate::editor_global::with_lua;
use crate::prefab::prefab_editor::{PrefabEditor, PrefabStage};
use crate::storage::editor_storage::load_game_by_name;
use engine_core::prelude::*;
use std::io;

impl PrefabStage {
    pub fn new(game_name: &str) -> Self {
        let mut game = load_prefab_game(game_name);

        with_lua(|lua| {
            AssetManager::init_editor_metadata(&mut game.asset_manager);
            ScriptManager::init_editor_services(&mut game.script_manager, lua);
        });

        Self {
            ecs: Ecs::default(),
            asset_manager: game.asset_manager,
            script_manager: game.script_manager,
            prefab_library: game.prefab_library,
        }
    }

    pub fn ctx_mut(&mut self) -> ServicesCtxMut<'_> {
        ServicesCtxMut {
            ecs: &mut self.ecs,
            world: None,
            asset_manager: &mut self.asset_manager,
            script_manager: &mut self.script_manager,
            prefab_library: &self.prefab_library,
        }
    }
}

impl PrefabEditor {
    pub fn open_existing(game_name: &str, prefab: PrefabAsset) -> (Self, PrefabStage) {
        let mut stage = PrefabStage::new(game_name);
        let root = {
            let mut game_ctx = stage.ctx_mut();
            instantiate_prefab(&mut game_ctx, &prefab, Vec2::ZERO, None)
        };

        let mut editor = Self::new(prefab.id, prefab.name.clone(), Some(prefab));
        editor.set_selected_entity(Some(root));
        editor.root_entity = Some(root);
        (editor, stage)
    }

    pub fn save_to_disk(
        &mut self,
        game_name: &str,
        game_ctx: &mut ServicesCtxMut,
    ) -> io::Result<Option<PrefabAsset>> {
        let Some(root) = self.root_entity else {
            return Ok(None);
        };

        let prefab = capture_prefab_with_existing(
            game_ctx.ecs,
            root,
            self.prefab_id,
            self.prefab_name.clone(),
            self.loaded_prefab.as_ref(),
        );
        save_prefab(game_name, &prefab)?;
        self.prefab_name = prefab.name.clone();
        self.loaded_prefab = Some(prefab.clone());
        Ok(Some(prefab))
    }

    pub fn set_name(&mut self, name: String) {
        self.prefab_name = name;
    }
}

fn load_prefab_game(game_name: &str) -> Game {
    load_game_by_name(game_name).unwrap_or_else(|_| Game {
        name: game_name.to_string(),
        ..Default::default()
    })
}
