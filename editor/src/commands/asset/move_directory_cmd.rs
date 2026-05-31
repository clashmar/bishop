use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct MoveDirectoryCmd {
    old_full_path: UserPath,
    new_full_path: PathBuf,
    pub(crate) saved_rewrites: Option<Vec<(AssetKey, PathBuf)>>,
}

impl MoveDirectoryCmd {
    pub fn new(old_full_path: impl Into<UserPath>, new_full_path: impl Into<PathBuf>) -> Self {
        Self {
            old_full_path: old_full_path.into(),
            new_full_path: new_full_path.into(),
            saved_rewrites: None,
        }
    }

    pub fn perform(&mut self) -> std::io::Result<()> {
        if is_protected_path(&self.old_full_path, &resources_folder_current()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot move engine-managed folders.",
            ));
        }
        if self.new_full_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "destination already exists",
            ));
        }
        if self.new_full_path.starts_with(&self.old_full_path) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot move a directory into itself",
            ));
        }
        fs::rename(&self.old_full_path, &self.new_full_path)?;

        with_editor(|editor| -> std::io::Result<()> {
            let registry = &editor.game.asset_registry;
            let old_prefix = self
                .old_full_path
                .strip_prefix(resources_folder_current())
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "old path is not under resources",
                    )
                })?;
            let new_prefix = self
                .new_full_path
                .strip_prefix(resources_folder_current())
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "new path is not under resources",
                    )
                })?;

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
            Ok(())
        })?;

        Ok(())
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

impl EditorCommand for MoveDirectoryCmd {
    fn execute(&mut self) {
        if let Err(e) = self.perform() {
            push_toast(format!("Move directory failed: {e}"), 3.0);
        }
    }

    fn undo(&mut self) {
        if self.saved_rewrites.is_none() {
            return;
        }

        if let Err(e) = fs::rename(&self.new_full_path, &self.old_full_path) {
            push_toast(format!("Undo move directory failed: {e}"), 3.0);
            return;
        }

        let result: std::io::Result<()> = with_editor(|editor| {
            let Some(old_prefix) = self
                .old_full_path
                .strip_prefix(resources_folder_current())
                .ok()
            else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "old path is not under resources",
                ));
            };
            let Some(new_prefix) = self
                .new_full_path
                .strip_prefix(resources_folder_current())
                .ok()
            else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "new path is not under resources",
                ));
            };

            let rewrites = self.saved_rewrites.as_ref().unwrap();
            for (key, new_path) in rewrites {
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
            Ok(())
        });

        if result.is_ok() {
            self.saved_rewrites.take();
        }
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
