use crate::app::*;
use crate::gui::modals::export_options::ExportOptionsModal;
use crate::gui::modals::export_overwrite::{stage_export_overwrite_message, ExportOverwriteModal};
use crate::gui::modals::export_result::ExportResultModal;
use crate::gui::modals::unsaved_exit::UnsavedExitModal;
use crate::gui::modals::ModalHandler;
use crate::prefab::reconcile_recent_prefab_ids;
use crate::prefab::palette::{PrefabPaletteState, PREFAB_PALETTE_RECENT_CAP};
use crate::storage::game_io::save_game;
use crate::storage::menus::{load_menus, save_menu};
use crate::storage::prefab_palettes::{load_prefab_palette_state, save_prefab_palette_state};
use crate::storage::export_dispatch::{export_game_for_target, export_target_path_for, ExportTarget, PendingExport};
use crate::storage::web_preview::{open_browser, open_folder, LocalPreview};
use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::logging::{omni_error};
use engine_core::menu::MenuTemplate;
use engine_core::storage::resources_folder_current;
use engine_core::ui::{Toast};
use std::path::{Path, PathBuf};
use std::io;

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
                omni_error!("Could not load prefab palette state: {error}");
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
            omni_error!("Could not save prefab palette state: {error}");
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

        let prefab_palette_saved = self.save_prefab_palette_state();

        if let Err(e) = save_game(&self.game) {
            omni_error!("Could not save game: {}.", e)
        } else if prefab_palette_saved {
            self.save_menus();
            self.toast = Some(Toast::new("Saved", 2.5));
        }

        self.update_save_state_hash();
    }

    /// Saves all menu templates to disk.
    pub fn save_menus(&self) {
        for template in &self.menu_editor.templates {
            if let Err(e) = save_menu(template) {
                omni_error!("Could not save menu '{}': {}", template.id, e);
            }
        }
    }

    /// Loads all menu templates from disk.
    pub fn load_menus(&mut self) {
        let templates = load_menus();
        self.menu_editor.set_templates(templates);
    }

    pub(crate) fn begin_export(&mut self, ctx: &mut WgpuContext) {
        ExportOptionsModal.open(self, ctx);
    }

    pub(crate) fn select_export_target(&mut self, ctx: &mut WgpuContext, target: ExportTarget) {
        use rfd::FileDialog;

        let Some(dest_root) = FileDialog::new()
            .set_title("Select destination folder for export:")
            .pick_folder()
        else {
            return;
        };
        let target_path = export_target_path_for(&dest_root, &self.game, target);
        if target_path.exists() {
            self.pending_export = Some(PendingExport { dest_root, target });
            let target_name = target_path.file_name().and_then(|n| n.to_str()).unwrap_or("export");
            stage_export_overwrite_message(format!("Overwrite existing export '{target_name}'?"));
            ExportOverwriteModal.open(self, ctx);
            return;
        }
        self.finish_export(ctx, &dest_root, target, false);
    }

    fn save_export_snapshot(&mut self) -> Result<PathBuf, String> {
        if matches!(self.mode, EditorMode::Prefab(_)) && !self.active_prefab_is_clean() {
            return Err("Save or discard staged prefab changes before exporting.".to_owned());
        }
        persist_export_snapshot(&self.game, &self.menu_editor.templates, save_game, save_menu)?;
        self.update_save_state_hash();
        Ok(resources_folder_current())
    }

    pub(crate) fn finish_export(&mut self, ctx: &WgpuContext, dest_root: &Path, target: ExportTarget, overwrite: bool) {
        let result = self.save_export_snapshot().and_then(|resources| {
            let runtime = crate::storage::web_preview::player_runtime_directory();
            export_game_for_target(dest_root, &resources, &runtime, &self.game, target, overwrite)
                .map_err(|error| format!("Could not export: {error}"))
        });
        match result {
            Ok(path) => {
                self.toast = Some(Toast::new(format!("Exported to: {}", path.display()), 2.5));
                if target == ExportTarget::Web {
                    self.web_preview = None;
                    self.last_web_export = Some(path);
                    ExportResultModal.open(self, ctx);
                }
            }
            Err(error) => {
                omni_error!("Export failed: {error}");
                self.toast = Some(Toast::new(format!("Export failed: {error}"), 4.0));
            }
        }
    }

    pub(crate) fn open_web_export_preview(&mut self) {
        let Some(site) = self.last_web_export.as_deref() else { return; };
        let result = match self.web_preview.as_ref() {
            Some(preview) => open_browser(&preview.url()),
            None => LocalPreview::start(site).and_then(|preview| {
                let opened = open_browser(&preview.url());
                if opened.is_ok() {
                    self.web_preview = Some(preview);
                }
                opened
            }),
        };
        if let Err(error) = result {
            self.web_preview = None;
            omni_error!("Web preview failed: {error}");
            self.toast = Some(Toast::new(format!("Web preview failed: {error}"), 4.0));
        }
    }

    pub(crate) fn open_web_export_folder(&mut self) {
        let Some(site) = self.last_web_export.as_deref() else { return; };
        if let Err(error) = open_folder(site) {
            omni_error!("Could not open export folder: {error}");
            self.toast = Some(Toast::new(format!("Could not open folder: {error}"), 4.0));
        }
    }

    pub(crate) fn compute_editor_state_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        let game_ron = ron::to_string(&self.game).unwrap_or_default();
        game_ron.hash(&mut hasher);

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

        use crate::storage::sound_presets::current_sound_preset_library;
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

fn persist_export_snapshot(
    game: &engine_core::game::Game,
    templates: &[MenuTemplate],
    save_game_fn: impl FnOnce(&engine_core::game::Game) -> io::Result<()>,
    mut save_menu_fn: impl FnMut(&MenuTemplate) -> io::Result<()>,
) -> Result<(), String> {
    save_game_fn(game).map_err(|error| format!("Could not save game for export: {error}"))?;
    for template in templates {
        save_menu_fn(template).map_err(|error| {
            format!("Could not save menu '{}' for export: {error}", template.id)
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_menu_snapshot_blocks_export_instead_of_reporting_saved() {
        use engine_core::menu::MenuTemplate;
        use std::cell::Cell;
        use std::io;

        let game = engine_core::game::Game::with_name("Unsaved Game");
        let templates = vec![MenuTemplate::new("unsaved-title".to_owned())];
        let game_saved = Cell::new(false);
        let result = persist_export_snapshot(&game, &templates, |_| {
            game_saved.set(true);
            Ok(())
        }, |_| Err(io::Error::other("readonly menus folder")));
        assert!(game_saved.get());
        assert!(result.unwrap_err().contains("unsaved-title"));

        let result = persist_export_snapshot(&game, &templates, |_| {
            Err(io::Error::other("read-only game"))
        }, |_| panic!("menus cannot be saved after a game save failure"));
        assert!(result.unwrap_err().contains("Could not save game"));
    }

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
