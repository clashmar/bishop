use crate::app::*;
use crate::storage::editor_storage::*;
use crate::storage::export::{export_game, export_target_path, PendingExport};
use bishop::prelude::*;
use engine_core::prelude::*;

impl Editor {
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

        if let Err(e) = save_game(&self.game) {
            onscreen_error!("Could not save game: {}.", e)
        } else if palette_saved {
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
                self.pending_export = Some(PendingExport {
                    dest_root,
                });
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
