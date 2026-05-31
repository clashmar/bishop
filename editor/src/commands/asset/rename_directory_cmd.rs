use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Undoable command that renames a directory tree and remaps affected registry paths.
#[derive(Debug)]
pub struct RenameDirectoryCmd {
    old_full_path: UserPath,
    new_full_path: PathBuf,
    saved_rewrites: Option<Vec<(AssetKey, PathBuf)>>,
}

impl RenameDirectoryCmd {
    /// Creates a rename-directory command from `old_full_path` to `new_full_path`.
    pub fn new(old_full_path: impl Into<UserPath>, new_full_path: impl Into<PathBuf>) -> Self {
        Self {
            old_full_path: old_full_path.into(),
            new_full_path: new_full_path.into(),
            saved_rewrites: None,
        }
    }

    fn collect_rewrites(
        registry: &AssetRegistry,
        old_prefix: &Path,
        new_prefix: &Path,
    ) -> Vec<(AssetKey, PathBuf)> {
        registry
            .records_under_prefix(old_prefix)
            .into_iter()
            .filter_map(|(key, record)| {
                record
                    .path
                    .strip_prefix(old_prefix)
                    .ok()
                    .map(|suffix| (key, new_prefix.join(suffix)))
            })
            .collect()
    }
}

impl EditorCommand for RenameDirectoryCmd {
    fn execute(&mut self) {
        if is_protected_path(&self.old_full_path, &resources_folder_current()) {
            push_toast("Cannot rename engine-managed folders.", 3.0);
            return;
        }

        if let Err(e) = fs::rename(&self.old_full_path, &self.new_full_path) {
            push_toast(format!("Rename directory failed: {e}"), 3.0);
            return;
        }

        with_editor(|editor| {
            let registry = &editor.game.asset_registry;
            let Some(old_prefix) = self
                .old_full_path
                .strip_prefix(resources_folder_current())
                .ok()
            else {
                push_toast("Rename directory: could not compute registry prefix", 3.0);
                return;
            };
            let Some(new_prefix) = self
                .new_full_path
                .strip_prefix(resources_folder_current())
                .ok()
            else {
                push_toast("Rename directory: could not compute registry prefix", 3.0);
                return;
            };

            let rewrites = Self::collect_rewrites(registry, old_prefix, new_prefix);
            for (key, new_path) in &rewrites {
                if let Err(e) = editor
                    .game
                    .asset_registry
                    .replace_record(*key, AssetRecord::new(new_path.clone()))
                {
                    push_toast(format!("Registry remap failed for {key:?}: {e}"), 3.0);
                }
            }
            self.saved_rewrites = Some(rewrites);
            editor.save();
        });
    }

    fn undo(&mut self) {
        let Some(rewrites) = self.saved_rewrites.take() else {
            return;
        };

        if let Err(e) = fs::rename(&self.new_full_path, &self.old_full_path) {
            push_toast(format!("Undo rename directory failed: {e}"), 3.0);
            self.saved_rewrites = Some(rewrites);
            return;
        }

        with_editor(|editor| {
            let Some(old_prefix) = self
                .old_full_path
                .strip_prefix(resources_folder_current())
                .ok()
            else {
                return;
            };
            let Some(new_prefix) = self
                .new_full_path
                .strip_prefix(resources_folder_current())
                .ok()
            else {
                return;
            };

            for (key, new_path) in &rewrites {
                let old_path = new_path
                    .strip_prefix(new_prefix)
                    .ok()
                    .map(|suffix| old_prefix.join(suffix));
                let Some(old_path) = old_path else { continue };
                if let Err(e) = editor
                    .game
                    .asset_registry
                    .replace_record(*key, AssetRecord::new(old_path))
                {
                    push_toast(format!("Undo registry remap failed for {key:?}: {e}"), 3.0);
                }
            }
            editor.save();
        });
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
