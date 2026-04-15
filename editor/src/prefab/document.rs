use crate::editor_global::with_lua;
use crate::prefab::prefab_editor::{
    PrefabEditor, PrefabRoomSyncState, PrefabStage, StagedPrefabState,
};
#[cfg(test)]
use crate::storage::editor_storage::load_game_by_name;
use engine_core::prelude::*;
use std::io;

macro_rules! for_each_prefab_asset_manager {
    ($callback:ident, $($args:tt)*) => {{
        $callback!($($args)*, sprite_manager);
        $callback!($($args)*, script_manager);
    }};
}

macro_rules! snapshot_prefab_asset_manager {
    ($game:expr, $stage:expr, $field:ident) => {
        $stage.$field = $game.$field.editor_metadata_snapshot();
    };
}

macro_rules! merge_prefab_asset_manager {
    ($game:expr, $stage:expr, $field:ident) => {
        $game.$field.merge_editor_metadata_from(&$stage.$field)?;
    };
}

impl PrefabStage {
    #[cfg(test)]
    /// Loads a prefab stage from the persisted game on disk.
    pub fn new(game_name: &str) -> Self {
        let game = load_prefab_game(game_name);
        Self::from_editor_services(&game)
    }

    /// Builds an isolated prefab stage from the live editor game services.
    pub fn from_editor_services(game: &Game) -> Self {
        let mut stage = Self {
            ecs: Ecs::default(),
            sprite_manager: SpriteManager::default(),
            script_manager: ScriptManager::default(),
            prefab_library: game.prefab_library.clone(),
        };
        for_each_prefab_asset_manager!(snapshot_prefab_asset_manager, game, stage);

        with_lua(|lua| {
            SpriteManager::init_editor_metadata(&mut stage.sprite_manager);
            ScriptManager::init_editor_services(&mut stage.script_manager, lua);
        });

        stage
    }

    /// Merges staged editor metadata back into the live game services.
    pub fn sync_editor_services(&self, game: &mut Game) -> io::Result<()> {
        for_each_prefab_asset_manager!(merge_prefab_asset_manager, game, self);
        Ok(())
    }

    pub fn ctx_mut(&mut self) -> ServicesCtxMut<'_> {
        ServicesCtxMut {
            ecs: &mut self.ecs,
            world: None,
            sprite_manager: &mut self.sprite_manager,
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
}

#[cfg(test)]
fn load_prefab_game(game_name: &str) -> Game {
    load_game_by_name(game_name).unwrap_or_else(|_| Game {
        name: game_name.to_string(),
        ..Default::default()
    })
}
