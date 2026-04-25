use crate::app::Editor;
use crate::editor_assets::write_prefabs_lua;
use crate::prefab::prefab_editor::StagedPrefabState;
use crate::storage::editor_storage::{collect_prefab_names, save_game};
use engine_core::prelude::*;
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InitialPrefabSaveTarget {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
}

pub(crate) fn derive_initial_prefab_save_target(
    path: PathBuf,
) -> Result<InitialPrefabSaveTarget, String> {
    if !path.starts_with(prefabs_folder()) {
        return Err("Selected prefab must be inside this project's prefabs folder.".to_string());
    }

    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| "Prefab name cannot be empty.".to_string())?
        .to_string();

    Ok(InitialPrefabSaveTarget { name, path })
}

pub(crate) fn pick_initial_prefab_save_path(suggested_name: &str) -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(result) = crate::prefab::tests::take_test_prefab_save_picker_result() {
        return result;
    }

    rfd::FileDialog::new()
        .add_filter("Prefab", &[extensions::PREFAB])
        .set_directory(prefabs_folder())
        .set_file_name(format!("{suggested_name}.{}", extensions::PREFAB))
        .save_file()
}

impl Editor {
    pub(crate) fn resolve_initial_prefab_save_target(
        &mut self,
        path: PathBuf,
    ) -> Option<InitialPrefabSaveTarget> {
        let target = match derive_initial_prefab_save_target(path) {
            Ok(target) => target,
            Err(message) => {
                self.toast = Some(Toast::new(message, 2.5));
                return None;
            }
        };

        let duplicate_exists = self
            .game
            .prefab_manager
            .prefabs
            .values()
            .any(|prefab| prefab.name == target.name);
        if duplicate_exists {
            self.toast = Some(Toast::new(
                format!("A prefab named \"{}\" already exists.", target.name),
                2.5,
            ));
            return None;
        }

        Some(target)
    }

    pub(crate) fn pick_initial_prefab_save_target(
        &mut self,
        suggested_name: &str,
    ) -> Option<InitialPrefabSaveTarget> {
        let path = pick_initial_prefab_save_path(suggested_name)?;
        self.resolve_initial_prefab_save_target(path)
    }

    pub fn save_active_prefab(&mut self) {
        if self.is_blank_prefab_mode() {
            self.toast = Some(Toast::new("Blank prefab sessions cannot be saved.", 2.5));
            return;
        }

        let Some(staged_state) = self.active_prefab_staged_state() else {
            return;
        };

        match staged_state {
            StagedPrefabState::PrefabAsset(prefab) => {
                self.commit_prefab_asset_save(prefab);
            }
            StagedPrefabState::Empty => {
                self.toast = Some(Toast::new("Prefab is empty", 2.5));
            }
        }
    }

    pub(crate) fn confirm_empty_prefab_save_delete(&mut self) {
        self.commit_prefab_delete();
    }

    pub(crate) fn commit_prefab_asset_save(&mut self, prefab: PrefabAsset) -> bool {
        if self.prefab_editor.is_none() {
            return false;
        }
        let prefab = canonical_prefab_asset(&prefab);
        let root_entity = self
            .prefab_editor
            .as_ref()
            .and_then(|prefab_editor| prefab_editor.root_entity);

        if let (Some(root_entity), Some(prefab_stage)) = (root_entity, self.prefab_stage.as_mut()) {
            super::room_sync::sync_prefab_stage_instance_metadata(
                &mut prefab_stage.ecs,
                root_entity,
                &prefab,
            );
        }

        if let Some(prefab_stage) = self.prefab_stage.as_ref() {
            if let Err(error) = prefab_stage.sync_editor_services(&mut self.game) {
                onscreen_error!("Could not save prefab: {error}");
                return false;
            }
        }

        let saved_prefab = match self.game.prefab_manager.save_prefab_and_sync(
            &self.game.name,
            &mut self.game.asset_registry,
            &prefab,
            None,
        ) {
            Ok(prefab) => prefab,
            Err(error) => {
                onscreen_error!("Could not save prefab: {error}");
                return false;
            }
        };

        if let Err(error) = save_game(&self.game) {
            onscreen_error!("Could not save prefab metadata: {error}");
            return false;
        }

        let Some(prefab_editor) = self.prefab_editor.as_mut() else {
            return false;
        };
        prefab_editor.record_saved_prefab_asset(saved_prefab.clone());

        if let Some(prefab_stage) = self.prefab_stage.as_mut() {
            prefab_stage
                .prefab_manager
                .prefabs
                .insert(saved_prefab.id, saved_prefab.clone());
        }

        self.reconcile_prefab_room_state(StagedPrefabState::PrefabAsset(saved_prefab.clone()));

        if !self.promote_prefab_in_palette(saved_prefab.id) {
            return false;
        }

        self.toast = Some(Toast::new("Prefab saved", 2.5));
        self.update_save_state_hash();
        true
    }

    pub(crate) fn commit_prefab_delete(&mut self) {
        let Some(prefab_id) = self.active_persisted_prefab_id() else {
            return;
        };

        if let Err(error) = self.game.prefab_manager.delete_prefab(
            &self.game.name,
            &mut self.game.asset_registry,
            prefab_id,
        ) {
            onscreen_error!("Could not delete prefab: {error}");
            return;
        }

        if let Err(error) = sync_prefabs_lua_file(&self.game) {
            onscreen_error!("Could not write prefabs.lua: {error}");
            return;
        }

        if let Err(error) = save_game(&self.game) {
            onscreen_error!("Could not save prefab metadata: {error}");
            return;
        }

        if let Some(prefab_stage) = self.prefab_stage.as_mut() {
            prefab_stage.prefab_manager.prefabs.remove(&prefab_id);
        }

        if let Some(prefab_editor) = self.prefab_editor.as_mut() {
            prefab_editor.last_committed_prefab = StagedPrefabState::Empty;
        }

        self.reconcile_prefab_room_state(StagedPrefabState::Empty);
        if !self.remove_prefab_from_palette(prefab_id) {
            return;
        }

        self.update_save_state_hash();
        self.toast = Some(Toast::new("Prefab deleted", 2.5));
    }
}

pub(super) fn sync_prefabs_lua_file(game: &Game) -> io::Result<()> {
    let prefab_names = collect_prefab_names(&game.prefab_manager)?;
    write_prefabs_lua(&scripts_folder(), &prefab_names)
}

pub(super) fn load_prefab_asset_from_path(path: &Path) -> io::Result<PrefabAsset> {
    let ron = fs::read_to_string(path)?;
    let prefab = ron::from_str(&ron).map_err(|error| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Could not parse prefab '{}': {error}", path.display()),
        )
    })?;

    validate_prefab(&prefab)?;
    Ok(prefab)
}
