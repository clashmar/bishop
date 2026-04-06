use crate::editor_global::with_lua;
use crate::prefab::prefab_editor::{
    PrefabEditor, PrefabRoomSyncState, PrefabStage, StagedPrefabState,
};
#[cfg(test)]
use crate::storage::editor_storage::load_game_by_name;
use engine_core::prelude::*;
use std::io;

impl PrefabStage {
    #[cfg(test)]
    /// Loads a prefab stage from the persisted game on disk.
    pub fn new(game_name: &str) -> Self {
        let game = load_prefab_game(game_name);
        Self::from_editor_services(&game)
    }

    /// Builds an isolated prefab stage from the live editor game services.
    pub fn from_editor_services(game: &Game) -> Self {
        let mut asset_manager = game.asset_manager.editor_metadata_snapshot();
        let mut script_manager = game.script_manager.editor_metadata_snapshot();

        with_lua(|lua| {
            AssetManager::init_editor_metadata(&mut asset_manager);
            ScriptManager::init_editor_services(&mut script_manager, lua);
        });

        Self {
            ecs: Ecs::default(),
            asset_manager,
            script_manager,
            prefab_library: game.prefab_library.clone(),
        }
    }

    /// Merges staged editor metadata back into the live game services.
    pub fn sync_editor_services(&self, game: &mut Game) -> io::Result<()> {
        game.asset_manager.merge_editor_metadata_from(&self.asset_manager)?;
        game.script_manager
            .merge_editor_metadata_from(&self.script_manager)?;
        Ok(())
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
    /// Opens a prefab editor using live editor game services as the stage seed.
    pub fn open_existing_from_game(
        game: &Game,
        prefab: PrefabAsset,
        last_room_synced_state: PrefabRoomSyncState,
    ) -> (Self, PrefabStage) {
        let mut stage = PrefabStage::from_editor_services(game);
        let root = {
            let mut game_ctx = stage.ctx_mut();
            instantiate_prefab(&mut game_ctx, &prefab, Vec2::ZERO, None)
        };

        let mut editor = Self::new(
            prefab.id,
            prefab.name.clone(),
            StagedPrefabState::PrefabAsset(prefab),
            last_room_synced_state,
        );
        editor.set_selected_entity(Some(root));
        editor.root_entity = Some(root);
        (editor, stage)
    }

    pub(crate) fn staged_prefab_state(
        &mut self,
        game_ctx: &mut ServicesCtxMut,
    ) -> StagedPrefabState {
        let Some(root) = self.root_entity else {
            return StagedPrefabState::Empty;
        };

        StagedPrefabState::PrefabAsset(capture_prefab_with_existing(
            game_ctx.ecs,
            root,
            self.prefab_id,
            self.prefab_name.clone(),
            self.committed_prefab_asset(),
        ))
    }

    pub(crate) fn save_prefab_asset(
        &mut self,
        game_name: &str,
        prefab: &PrefabAsset,
    ) -> io::Result<()> {
        save_prefab(game_name, prefab)?;
        self.prefab_name = prefab.name.clone();
        self.last_committed_prefab = StagedPrefabState::PrefabAsset(prefab.clone());
        Ok(())
    }

    pub fn set_name(&mut self, name: String) {
        self.prefab_name = name;
    }
}

#[cfg(test)]
fn load_prefab_game(game_name: &str) -> Game {
    load_game_by_name(game_name).unwrap_or_else(|_| Game {
        name: game_name.to_string(),
        ..Default::default()
    })
}

#[cfg(test)]
/// Opens a prefab editor using persisted game data loaded from disk.
pub fn open_existing(
    game_name: &str,
    prefab: PrefabAsset,
    last_room_synced_state: PrefabRoomSyncState,
) -> (Self, PrefabStage) {
    let game = load_prefab_game(game_name);
    Self::open_existing_from_game(&game, prefab, last_room_synced_state)
}
