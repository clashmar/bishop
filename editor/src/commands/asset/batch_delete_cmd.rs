use crate::app::EditorMode;
use crate::commands::asset::{DeleteAssetCmd, DeleteDirectoryCmd, DeleteUnregisteredFileCmd};
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;
use std::path::PathBuf;

/// A single resource selected for deletion.
#[derive(Clone, Debug)]
pub enum DeleteTarget {
    RegisteredFile { key: AssetKey, full_path: PathBuf },
    UnregisteredFile(PathBuf),
    Directory(UserPath),
}

impl DeleteTarget {
    fn path(&self) -> &std::path::Path {
        match self {
            DeleteTarget::RegisteredFile { full_path, .. } => full_path,
            DeleteTarget::UnregisteredFile(path) => path,
            DeleteTarget::Directory(user_path) => user_path.as_ref(),
        }
    }
}

#[derive(Debug)]
enum ExecutedDelete {
    Asset(DeleteAssetCmd),
    UnregisteredFile(DeleteUnregisteredFileCmd),
    Directory(DeleteDirectoryCmd),
}

/// Undo-able command for deleting multiple files and directories.
///
/// - Deduplicates overlapping selections (a directory subsumes any selected
///   files or sub-directories inside it).
/// - Best-effort: individual failures are reported, and undo only restores
///   the successfully deleted items.
#[derive(Debug)]
pub struct BatchDeleteCmd {
    targets: Vec<DeleteTarget>,
    executed: Vec<ExecutedDelete>,
}

impl BatchDeleteCmd {
    pub fn new(targets: Vec<DeleteTarget>) -> Self {
        Self {
            targets,
            executed: Vec::new(),
        }
    }

    fn deduplicate_targets(targets: &[DeleteTarget]) -> Vec<DeleteTarget> {
        let (mut dirs, files): (Vec<_>, Vec<_>) = targets
            .iter()
            .cloned()
            .partition(|t| matches!(t, DeleteTarget::Directory(_)));

        // Sort directories by depth (deepest first) so parent containment works
        // correctly when we scan in order.
        dirs.sort_by_key(|d| std::cmp::Reverse(d.path().components().count()));

        let mut kept_dirs: Vec<DeleteTarget> = Vec::new();
        for dir in dirs {
            let dir_path = dir.path();
            let is_contained = kept_dirs
                .iter()
                .any(|kept| dir_path.starts_with(kept.path()));
            if !is_contained {
                kept_dirs.push(dir);
            }
        }

        let mut kept_files: Vec<DeleteTarget> = Vec::new();
        for file in files {
            let file_path = file.path();
            let is_contained = kept_dirs
                .iter()
                .any(|kept| file_path.starts_with(kept.path()));
            if !is_contained {
                kept_files.push(file);
            }
        }

        kept_dirs.into_iter().chain(kept_files).collect()
    }
}

impl EditorCommand for BatchDeleteCmd {
    fn execute(&mut self) {
        let targets = Self::deduplicate_targets(&self.targets);
        let mut executed: Vec<ExecutedDelete> = Vec::new();
        let mut failures = 0usize;

        with_editor(|editor| {
            for target in targets {
                match target {
                    DeleteTarget::RegisteredFile { key, .. } => {
                        let mut cmd = DeleteAssetCmd::new(key);
                        if cmd.perform(editor) {
                            executed.push(ExecutedDelete::Asset(cmd));
                        } else {
                            failures += 1;
                        }
                    }
                    DeleteTarget::UnregisteredFile(path) => {
                        let mut cmd = DeleteUnregisteredFileCmd::new(path);
                        if cmd.perform() {
                            executed.push(ExecutedDelete::UnregisteredFile(cmd));
                        } else {
                            failures += 1;
                        }
                    }
                    DeleteTarget::Directory(user_path) => {
                        let mut cmd = DeleteDirectoryCmd::new(user_path);
                        if cmd.perform(editor) {
                            executed.push(ExecutedDelete::Directory(cmd));
                        } else {
                            failures += 1;
                        }
                    }
                }
            }

            if !executed.is_empty() {
                editor.save();
            }
        });

        self.executed = executed;

        if failures > 0 {
            let suffix = if failures == 1 { "" } else { "s" };
            push_toast(format!("Could not delete {failures} item{suffix}"), 3.0);
        }
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let mut any_restored = false;
            for executed in &mut self.executed {
                match executed {
                    ExecutedDelete::Asset(cmd) => any_restored |= cmd.restore(editor),
                    ExecutedDelete::UnregisteredFile(cmd) => any_restored |= cmd.restore(),
                    ExecutedDelete::Directory(cmd) => any_restored |= cmd.restore(editor),
                }
            }
            if any_restored {
                editor.save();
            }
        });
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
