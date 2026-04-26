// editor/src/commands/asset/delete_directory_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Snapshot of a single file for undo restoration.
#[derive(Clone, Debug)]
struct SavedFile {
    relative_path: PathBuf,
    bytes: Vec<u8>,
}

/// Undoable command that deletes a directory tree and removes affected registry records.
#[derive(Debug)]
pub struct DeleteDirectoryCmd {
    full_path: PathBuf,
    saved_files: Option<Vec<SavedFile>>,
    saved_records: Option<Vec<(AssetKey, AssetRecord)>>,
}

impl DeleteDirectoryCmd {
    /// Creates a delete-directory command for the given full path.
    pub fn new(full_path: impl Into<PathBuf>) -> Self {
        Self {
            full_path: full_path.into(),
            saved_files: None,
            saved_records: None,
        }
    }

    fn collect_files(root: &Path) -> Vec<SavedFile> {
        let mut result = Vec::new();
        Self::visit_files(root, root, &mut result);
        result
    }

    fn visit_files(root: &Path, dir: &Path, out: &mut Vec<SavedFile>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Self::visit_files(root, &path, out);
            } else {
                let Ok(bytes) = fs::read(&path) else { continue };
                let Ok(relative) = path.strip_prefix(root) else {
                    continue;
                };
                out.push(SavedFile {
                    relative_path: relative.to_path_buf(),
                    bytes,
                });
            }
        }
    }
}

impl EditorCommand for DeleteDirectoryCmd {
    fn execute(&mut self) {
        let files = Self::collect_files(&self.full_path);

        with_editor(|editor| {
            let Some(prefix) = self.full_path.strip_prefix(resources_folder_current()).ok() else {
                push_toast("Delete directory: could not compute registry prefix", 3.0);
                return;
            };

            let affected = editor.game.asset_registry.records_under_prefix(prefix);

            if let Err(e) = fs::remove_dir_all(&self.full_path) {
                push_toast(format!("Delete directory failed: {e}"), 3.0);
                return;
            }

            for (key, _) in &affected {
                editor.game.asset_registry.remove_record(*key);
            }

            self.saved_files = Some(files);
            self.saved_records = Some(affected);
            editor.save();
        });
    }

    fn undo(&mut self) {
        let Some(files) = self.saved_files.take() else {
            return;
        };
        let Some(records) = self.saved_records.take() else {
            self.saved_files = Some(files);
            return;
        };

        if let Some(parent) = self.full_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::create_dir_all(&self.full_path);

        for saved in &files {
            let file_path = self.full_path.join(&saved.relative_path);
            if let Some(file_parent) = file_path.parent() {
                let _ = fs::create_dir_all(file_parent);
            }
            let _ = fs::write(&file_path, &saved.bytes);
        }

        with_editor(|editor| {
            for (key, record) in &records {
                if let Err(e) = editor.game.asset_registry.insert(*key, record.clone()) {
                    push_toast(format!("Undo delete directory: could not restore registry record for {key:?}: {e}"), 3.0);
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
