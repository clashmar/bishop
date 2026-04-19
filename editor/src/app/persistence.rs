use crate::app::*;
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
        let recent_prefab_ids = reconcile_recent_prefab_ids(
            state.recent_prefab_ids,
            &self.game.prefab_library,
        );
        let active_prefab_id = state
            .active_prefab_id
            .filter(|prefab_id| self.game.prefab_library.prefabs.contains_key(prefab_id))
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
            .reconcile_prefab_palette(&self.game.prefab_library);

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
                .load_prefab_palette_state(&self.game.prefab_library, state),
            Err(error) => {
                onscreen_error!("Could not load prefab palette state: {error}");
                self.room_editor.load_prefab_palette_state(
                    &self.game.prefab_library,
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
        if !self.game.prefab_library.prefabs.contains_key(&prefab_id) {
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
        if !self.game.prefab_library.prefabs.contains_key(&prefab_id) {
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
                self.open_export_overwrite_modal(ctx, &target_path);
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
}
