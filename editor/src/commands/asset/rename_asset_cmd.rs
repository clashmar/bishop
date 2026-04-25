// editor/src/commands/asset/rename_asset_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub struct RenameAssetCmd {
    key: AssetKey,
    new_relative_path: PathBuf,
    old_relative_path: Option<PathBuf>,
    old_full_path: Option<PathBuf>,
    new_full_path: Option<PathBuf>,
}

impl RenameAssetCmd {
    pub fn new(key: AssetKey, new_relative_path: impl Into<PathBuf>) -> Self {
        Self {
            key,
            new_relative_path: new_relative_path.into(),
            old_relative_path: None,
            old_full_path: None,
            new_full_path: None,
        }
    }

    pub fn is_valid(&self) -> Option<String> {
        with_editor(|editor| {
            let registry = &editor.game.asset_registry;
            if registry.record(self.key).is_none() {
                return Some(format!("Asset {:?} not found in registry", self.key));
            }
            let kind = AssetRegistry::kind_for_key(self.key);
            let folder = AssetRegistry::asset_folder(kind);
            let canonical = folder.join(&self.new_relative_path);
            if let Some(owner) = registry.key_for_path(&canonical) {
                if owner != self.key {
                    return Some(format!(
                        "Path '{}' already registered to {:?}",
                        canonical.display(),
                        owner
                    ));
                }
            }
            None
        })
    }

    fn perform_rename(&mut self) -> io::Result<()> {
        let new_relative_path = self.new_relative_path.clone();
        let key = self.key;

        with_editor(|editor| {
            let old_record = editor
                .game
                .asset_registry
                .record(key)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "asset not in registry"))?;

            let kind = AssetRegistry::kind_for_key(key);
            let folder = AssetRegistry::asset_folder(kind);
            let old_relative = editor
                .game
                .asset_registry
                .relative_path(key)
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "no relative path for asset")
                })?;
            let old_full = resources_folder_current().join(&old_record.path);
            let new_canonical = folder.join(&new_relative_path);
            let new_full = resources_folder_current().join(&new_canonical);

            self.old_relative_path = Some(old_relative);
            self.old_full_path = Some(old_full.clone());
            self.new_full_path = Some(new_full.clone());

            if let Some(parent) = new_full.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::rename(&old_full, &new_full)?;

            editor
                .game
                .asset_registry
                .replace_record(key, AssetRecord::new(new_canonical))?;

            if let AssetKey::Prefab(_) = key {
                Self::sync_prefab_rename(editor, key);
            }

            editor.save();
            Ok(())
        })
    }

    fn sync_prefab_rename(editor: &mut crate::Editor, key: AssetKey) {
        let AssetKey::Prefab(prefab_id) = key else {
            return;
        };
        let Some(prefab) = editor.game.prefab_manager.prefabs.get(&prefab_id).cloned() else {
            return;
        };
        let new_stem = editor
            .game
            .asset_registry
            .relative_path(key)
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()));

        if let Some(name) = new_stem {
            let mut updated = prefab;
            updated.name = name;
            if let Err(e) = editor.game.prefab_manager.save_prefab_and_sync(
                &editor.game.name,
                &mut editor.game.asset_registry,
                &updated,
            ) {
                push_toast(format!("Prefab save failed: {e}"), 3.0);
            }
        }
    }
}

impl EditorCommand for RenameAssetCmd {
    fn execute(&mut self) {
        if let Some(err) = self.is_valid() {
            push_toast(err, 3.0);
            return;
        }
        if let Err(e) = self.perform_rename() {
            push_toast(format!("Rename failed: {e}"), 3.0);
        }
    }

    fn undo(&mut self) {
        let Some(old_relative) = self.old_relative_path.take() else {
            return;
        };
        let Some(old_full) = self.old_full_path.take() else {
            return;
        };
        let Some(new_full) = self.new_full_path.take() else {
            return;
        };
        let key = self.key;

        with_editor(|editor| {
            if new_full.exists() {
                if let Err(e) = fs::rename(&new_full, &old_full) {
                    push_toast(format!("Undo rename failed: {e}"), 3.0);
                    return;
                }
            }

            let kind = AssetRegistry::kind_for_key(key);
            let folder = AssetRegistry::asset_folder(kind);
            let canonical = folder.join(&old_relative);

            if let Err(e) = editor
                .game
                .asset_registry
                .replace_record(key, AssetRecord::new(canonical))
            {
                push_toast(format!("Undo registry update failed: {e}"), 3.0);
                return;
            }

            if let AssetKey::Prefab(_) = key {
                Self::sync_prefab_rename(editor, key);
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
