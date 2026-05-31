use crate::app::EditorMode;
use crate::commands::asset::{MoveDirectoryCmd, MoveFileCmd};
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;
use std::path::PathBuf;

/// A single target to move, either a file or a directory.
#[derive(Clone, Debug)]
pub enum MoveTarget {
    File {
        old_path: PathBuf,
        new_path: PathBuf,
        key: Option<AssetKey>,
    },
    Directory {
        old_path: UserPath,
        new_path: PathBuf,
    },
}

#[derive(Debug)]
enum ExecutedMove {
    File(MoveFileCmd),
    Directory(MoveDirectoryCmd),
}

/// A batch command that moves multiple files and/or directories.
#[derive(Debug)]
pub struct BatchMoveCmd {
    targets: Vec<MoveTarget>,
    executed: Vec<ExecutedMove>,
}

impl BatchMoveCmd {
    /// Creates a new batch move command with the given targets.
    pub fn new(targets: Vec<MoveTarget>) -> Self {
        Self {
            targets,
            executed: Vec::new(),
        }
    }

    fn deduplicate_targets(targets: &[MoveTarget]) -> Vec<MoveTarget> {
        let (mut dirs, files): (Vec<_>, Vec<_>) = targets
            .iter()
            .cloned()
            .partition(|t| matches!(t, MoveTarget::Directory { .. }));

        dirs.sort_by_key(|d| {
            let path = match d {
                MoveTarget::Directory { old_path, .. } => old_path.as_ref(),
                _ => unreachable!(),
            };
            std::cmp::Reverse(path.components().count())
        });

        let mut kept_dirs: Vec<MoveTarget> = Vec::new();
        for dir in dirs {
            let dir_path = match &dir {
                MoveTarget::Directory { old_path, .. } => old_path.as_ref(),
                _ => unreachable!(),
            };
            let is_contained = kept_dirs.iter().any(|kept| {
                let kept_path = match kept {
                    MoveTarget::Directory { old_path, .. } => old_path.as_ref(),
                    _ => unreachable!(),
                };
                kept_path.starts_with(dir_path)
            });
            if !is_contained {
                kept_dirs.push(dir);
            }
        }

        let mut kept_files: Vec<MoveTarget> = Vec::new();
        for file in files {
            let file_path = match &file {
                MoveTarget::File { old_path, .. } => old_path.as_path(),
                _ => unreachable!(),
            };
            let is_contained = kept_dirs.iter().any(|kept| {
                let kept_path = match kept {
                    MoveTarget::Directory { old_path, .. } => old_path.as_ref(),
                    _ => unreachable!(),
                };
                file_path.starts_with(kept_path)
            });
            if !is_contained {
                kept_files.push(file);
            }
        }

        kept_dirs.into_iter().chain(kept_files).collect()
    }
}

impl EditorCommand for BatchMoveCmd {
    fn execute(&mut self) {
        let targets = Self::deduplicate_targets(&self.targets);
        let mut executed: Vec<ExecutedMove> = Vec::new();
        let mut failures = 0usize;

        for target in targets {
            match target {
                MoveTarget::File {
                    old_path,
                    new_path,
                    key,
                } => {
                    let mut cmd = MoveFileCmd::new(old_path, new_path, key);
                    if cmd.perform().is_ok() {
                        executed.push(ExecutedMove::File(cmd));
                    } else {
                        failures += 1;
                    }
                }
                MoveTarget::Directory { old_path, new_path } => {
                    let mut cmd = MoveDirectoryCmd::new(old_path, new_path);
                    if cmd.perform().is_ok() {
                        executed.push(ExecutedMove::Directory(cmd));
                    } else {
                        failures += 1;
                    }
                }
            }
        }

        self.executed = executed;

        if failures > 0 {
            let suffix = if failures == 1 { "" } else { "s" };
            push_toast(format!("Could not move {failures} item{suffix}"), 3.0);
        }
    }

    fn undo(&mut self) {
        let mut any_restored = false;
        for executed in self.executed.iter_mut().rev() {
            match executed {
                ExecutedMove::File(cmd) => {
                    let had_state = cmd.key.is_none() || cmd.old_relative_path.is_some();
                    cmd.undo();
                    any_restored |= had_state;
                }
                ExecutedMove::Directory(cmd) => {
                    let had_state = cmd.saved_rewrites.is_some();
                    cmd.undo();
                    any_restored |= had_state;
                }
            }
        }
        if any_restored {
            with_editor(|editor| editor.save());
        }
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
