use crate::app::*;
use crate::gui::modals::export_overwrite::{stage_export_overwrite_message, ExportOverwriteModal};
use crate::gui::modals::unsaved_exit::UnsavedExitModal;
use crate::gui::modals::ModalHandler;
use crate::prefab::reconcile_recent_prefab_ids;
use crate::storage::editor_storage::*;
use crate::storage::export::{export_game, export_target_path, PendingExport};
use bishop::prelude::*;
use engine_core::prelude::*;

#[derive(Clone, Copy)]
enum PrefabPaletteRollbackMode {
    ExactSnapshot,
    ReconcileCurrentLibrary,
}

impl Editor {
    fn reconcile_prefab_palette_state_after_library_change(
        &self,
        state: PrefabPaletteState,
    ) -> PrefabPaletteState {
        let recent_prefab_ids =
            reconcile_recent_prefab_ids(state.recent_prefab_ids, &self.game.prefab_manager);
        let active_prefab_id = state
            .active_prefab_id
            .filter(|prefab_id| self.game.prefab_manager.prefabs.contains_key(prefab_id))
            .or_else(|| recent_prefab_ids.first().copied());

        PrefabPaletteState {
            active_prefab_id,
            recent_prefab_ids,
        }
    }

    fn commit_prefab_palette_change(
        &mut self,
        apply_change: impl FnOnce(&mut PrefabPaletteState),
        rollback_mode: PrefabPaletteRollbackMode,
    ) -> bool {
        let snapshot = self.room_editor.prefab_palette_state();
        let mut palette_state = snapshot.clone();

        apply_change(&mut palette_state);
        self.room_editor.active_prefab_id = palette_state.active_prefab_id;
        self.room_editor.recent_prefab_ids = palette_state.recent_prefab_ids;
        self.room_editor
            .reconcile_prefab_palette(&self.game.prefab_manager);

        if self.save_prefab_palette_state() {
            return true;
        }

        let rollback_state = match rollback_mode {
            PrefabPaletteRollbackMode::ExactSnapshot => snapshot,
            PrefabPaletteRollbackMode::ReconcileCurrentLibrary => {
                self.reconcile_prefab_palette_state_after_library_change(snapshot)
            }
        };
        self.room_editor.active_prefab_id = rollback_state.active_prefab_id;
        self.room_editor.recent_prefab_ids = rollback_state.recent_prefab_ids;
        false
    }

    pub(crate) fn load_prefab_palette_state(&mut self) {
        match load_prefab_palette_state(&self.game.name) {
            Ok(state) => self
                .room_editor
                .load_prefab_palette_state(&self.game.prefab_manager, state),
            Err(error) => {
                onscreen_error!("Could not load prefab palette state: {error}");
                self.room_editor.load_prefab_palette_state(
                    &self.game.prefab_manager,
                    PrefabPaletteState::default(),
                );
            }
        }
    }

    pub(crate) fn save_prefab_palette_state(&self) -> bool {
        if let Err(error) =
            save_prefab_palette_state(&self.game.name, &self.room_editor.prefab_palette_state())
        {
            onscreen_error!("Could not save prefab palette state: {error}");
            return false;
        }
        true
    }

    pub(crate) fn reconcile_prefab_palette_after_library_change(&mut self) -> bool {
        let palette_state = self.reconcile_prefab_palette_state_after_library_change(
            self.room_editor.prefab_palette_state(),
        );
        self.room_editor.active_prefab_id = palette_state.active_prefab_id;
        self.room_editor.recent_prefab_ids = palette_state.recent_prefab_ids;
        self.save_prefab_palette_state()
    }

    pub(crate) fn promote_prefab_in_palette(&mut self, prefab_id: PrefabId) -> bool {
        if !self.game.prefab_manager.prefabs.contains_key(&prefab_id) {
            return false;
        }

        self.commit_prefab_palette_change(
            |state| {
                state.active_prefab_id = Some(prefab_id);
                state.recent_prefab_ids.retain(|id| *id != prefab_id);
                state.recent_prefab_ids.insert(0, prefab_id);
                state.recent_prefab_ids.truncate(PREFAB_PALETTE_RECENT_CAP);
            },
            PrefabPaletteRollbackMode::ExactSnapshot,
        )
    }

    pub(crate) fn remove_prefab_from_palette(&mut self, prefab_id: PrefabId) -> bool {
        self.commit_prefab_palette_change(
            |state| {
                state.recent_prefab_ids.retain(|id| *id != prefab_id);
            },
            PrefabPaletteRollbackMode::ReconcileCurrentLibrary,
        )
    }

    pub(crate) fn activate_prefab(&mut self, prefab_id: PrefabId) -> bool {
        if !self.game.prefab_manager.prefabs.contains_key(&prefab_id) {
            return false;
        }

        self.room_editor.activate_prefab(prefab_id);
        self.save_prefab_palette_state()
    }

    pub fn save(&mut self) {
        if matches!(self.mode, EditorMode::Prefab(_)) {
            self.save_active_prefab();
            return;
        }

        let palette = &self.room_editor.tilemap_editor.tilemap_panel.palette;
        let palette_saved = if let Err(e) = save_palette(palette, &self.game.name) {
            onscreen_error!("Could not save palette: {e}");
            false
        } else {
            true
        };
        let prefab_palette_saved = self.save_prefab_palette_state();

        if let Err(e) = save_game(&self.game) {
            onscreen_error!("Could not save game: {}.", e)
        } else if palette_saved && prefab_palette_saved {
            self.save_menus();
            self.toast = Some(Toast::new("Saved", 2.5));
        }

        self.update_save_state_hash();
    }

    /// Saves all menu templates to disk.
    pub fn save_menus(&self) {
        for template in &self.menu_editor.templates {
            if let Err(e) = save_menu(template) {
                onscreen_error!("Could not save menu '{}': {}", template.id, e);
            }
        }
    }

    /// Loads all menu templates from disk.
    pub fn load_menus(&mut self) {
        let templates = load_menus();
        self.menu_editor.set_templates(templates);
    }

    pub(crate) fn begin_export(&mut self, ctx: &mut WgpuContext) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rfd::FileDialog;

            let Some(dest_root) = FileDialog::new()
                .set_title("Select destination folder for export:")
                .pick_folder()
            else {
                return;
            };

            let target_path = export_target_path(&dest_root, &self.game);
            if target_path.exists() {
                self.pending_export = Some(PendingExport { dest_root });
                let target_name = target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("export");
                let message = format!("Overwrite existing export '{target_name}'?");
                stage_export_overwrite_message(message);
                ExportOverwriteModal.open(self, ctx);
                return;
            }

            self.finish_export(&dest_root);
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.toast = Some(Toast::new("Folder picker unavailable in WASM", 2.5));
        }
    }

    pub(crate) fn finish_export(&mut self, dest_root: &std::path::Path) {
        match export_game(dest_root, &self.game) {
            Ok(path) => {
                self.toast = Some(Toast::new(format!("Exported to: {}", path.display()), 2.5));
            }
            Err(e) => {
                onscreen_error!("Export failed: {e}");
            }
        }
    }

    pub(crate) fn compute_editor_state_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        let game_ron = ron::to_string(&self.game).unwrap_or_default();
        game_ron.hash(&mut hasher);

        let palette = &self.room_editor.tilemap_editor.tilemap_panel.palette;
        let palette_ron = ron::to_string(palette).unwrap_or_default();
        palette_ron.hash(&mut hasher);

        let pps_ron = ron::to_string(&self.room_editor.prefab_palette_state()).unwrap_or_default();
        pps_ron.hash(&mut hasher);

        let mut menu_rons: Vec<_> = self
            .menu_editor
            .templates
            .iter()
            .filter_map(|t| ron::to_string(t).ok())
            .collect();
        menu_rons.sort();
        menu_rons.hash(&mut hasher);

        use crate::storage::sound_preset_storage::current_sound_preset_library;
        let sound_library = current_sound_preset_library();
        let sound_ron = ron::to_string(&sound_library).unwrap_or_default();
        sound_ron.hash(&mut hasher);

        hasher.finish()
    }

    pub(crate) fn update_save_state_hash(&mut self) {
        self.last_save_hash = self.compute_editor_state_hash();
    }

    pub(crate) fn is_dirty(&mut self) -> bool {
        self.compute_editor_state_hash() != self.last_save_hash || !self.active_prefab_is_clean()
    }

    pub(crate) fn update_handle_close_request(&mut self, ctx: &mut WgpuContext) {
        if !ctx.is_close_requested() || self.handling_close {
            return;
        }
        if !self.is_dirty() {
            ctx.set_exit_confirmed(true);
            return;
        }
        ctx.set_exit_confirmed(false);
        UnsavedExitModal.open(self, ctx);
        self.handling_close = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_state_hash_is_consistent_for_identical_state() {
        let editor = Editor::default();
        let hash1 = editor.compute_editor_state_hash();
        let hash2 = editor.compute_editor_state_hash();
        assert_eq!(
            hash1, hash2,
            "hash should be deterministic for the same state"
        );
    }

    #[test]
    fn editor_state_hash_changes_when_game_mutates() {
        let mut editor = Editor::default();
        let baseline = editor.compute_editor_state_hash();

        editor.game.name = "mutated".to_string();
        let mutated = editor.compute_editor_state_hash();

        assert_ne!(
            baseline, mutated,
            "hash should change when game state changes"
        );
    }

    #[test]
    fn update_save_state_hash_updates_last_save_hash() {
        let mut editor = Editor::default();
        assert_eq!(editor.last_save_hash, 0, "initial hash should be 0");

        editor.update_save_state_hash();
        let computed = editor.compute_editor_state_hash();

        assert_eq!(
            editor.last_save_hash, computed,
            "last_save_hash should match computed hash"
        );
    }

    #[test]
    fn is_dirty_returns_false_after_hash_update() {
        let mut editor = Editor::default();
        editor.update_save_state_hash();
        assert!(
            !editor.is_dirty(),
            "editor should not be dirty immediately after hash update"
        );
    }

    #[test]
    fn is_dirty_returns_true_after_game_mutation() {
        let mut editor = Editor::default();
        editor.update_save_state_hash();

        editor.game.name = "changed".to_string();
        assert!(
            editor.is_dirty(),
            "editor should be dirty after game mutation"
        );
    }
}
