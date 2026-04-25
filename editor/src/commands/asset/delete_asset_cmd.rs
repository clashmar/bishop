// editor/src/commands/asset/delete_asset_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct DeleteAssetCmd {
    key: AssetKey,
    saved_record: Option<AssetRecord>,
    saved_bytes: Option<Vec<u8>>,
    saved_full_path: Option<PathBuf>,
}

impl DeleteAssetCmd {
    pub fn new(key: AssetKey) -> Self {
        Self {
            key,
            saved_record: None,
            saved_bytes: None,
            saved_full_path: None,
        }
    }
}

impl EditorCommand for DeleteAssetCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let Some(record) = editor.game.asset_registry.record(self.key).cloned() else {
                push_toast(format!("Asset {:?} not found in registry", self.key), 3.0);
                return;
            };
            let full_path = resources_folder_current().join(&record.path);

            let bytes = match fs::read(&full_path) {
                Ok(b) => b,
                Err(e) => {
                    push_toast(format!("Could not read asset file: {e}"), 3.0);
                    return;
                }
            };

            self.saved_record = Some(record.clone());
            self.saved_bytes = Some(bytes);
            self.saved_full_path = Some(full_path.clone());

            if full_path.exists() {
                if let Err(e) = fs::remove_file(&full_path) {
                    push_toast(format!("Could not delete asset file: {e}"), 3.0);
                    return;
                }
            }

            editor.game.asset_registry.remove_record(self.key);

            if let AssetKey::Prefab(prefab_id) = self.key {
                let _ = editor.game.prefab_manager.prefabs.remove(&prefab_id);
            }

            editor.save();
        });
    }

    fn undo(&mut self) {
        let Some(record) = self.saved_record.take() else {
            return;
        };
        let Some(bytes) = self.saved_bytes.take() else {
            return;
        };
        let Some(full_path) = self.saved_full_path.take() else {
            return;
        };
        let key = self.key;

        with_editor(|editor| {
            if let Some(parent) = full_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(e) = fs::write(&full_path, &bytes) {
                push_toast(format!("Could not restore asset file: {e}"), 3.0);
                return;
            }

            if let Err(e) = editor.game.asset_registry.insert(key, record) {
                push_toast(format!("Could not restore registry record: {e}"), 3.0);
                return;
            }

            if let AssetKey::Prefab(prefab_id) = key {
                if let Some(prefab) = editor.game.prefab_manager.prefabs.get(&prefab_id).cloned() {
                    if let Err(e) = editor.game.prefab_manager.save_prefab_and_sync(
                        &editor.game.name,
                        &mut editor.game.asset_registry,
                        &prefab,
                    ) {
                        push_toast(format!("Could not restore prefab file: {e}"), 3.0);
                    }
                }
            }

            editor.save();
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Game
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
