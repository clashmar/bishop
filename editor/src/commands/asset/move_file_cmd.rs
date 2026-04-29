// editor/src/commands/asset/move_file_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub struct MoveFileCmd {
    old_full_path: PathBuf,
    new_full_path: PathBuf,
    pub(crate) key: Option<AssetKey>,
    pub(crate) old_relative_path: Option<PathBuf>,
}

impl MoveFileCmd {
    pub fn new(
        old_full_path: impl Into<PathBuf>,
        new_full_path: impl Into<PathBuf>,
        key: Option<AssetKey>,
    ) -> Self {
        Self {
            old_full_path: old_full_path.into(),
            new_full_path: new_full_path.into(),
            key,
            old_relative_path: None,
        }
    }

    pub fn perform(&mut self) -> io::Result<()> {
        if self.new_full_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "destination already exists",
            ));
        }

        let new_canonical = if let Some(key) = self.key {
            let kind = AssetRegistry::kind_for_key(key);
            let folder = AssetRegistry::asset_folder(kind);
            let new_relative = self
                .new_full_path
                .strip_prefix(resources_folder_current())
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "new path is not under resources",
                    )
                })?
                .strip_prefix(&folder)
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "new path is not under asset folder",
                    )
                })?;
            let new_canonical = folder.join(new_relative);

            with_editor(|editor| {
                if let Some(existing_key) = editor.game.asset_registry.key_for_path(&new_canonical)
                {
                    if existing_key != key {
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            "destination path is already registered to another asset",
                        ));
                    }
                }
                Ok::<(), io::Error>(())
            })?;

            Some((key, new_canonical))
        } else {
            None
        };

        fs::rename(&self.old_full_path, &self.new_full_path)?;

        if let Some((key, new_canonical)) = new_canonical {
            with_editor(|editor| {
                let old_relative =
                    editor
                        .game
                        .asset_registry
                        .relative_path(key)
                        .ok_or_else(|| {
                            io::Error::new(io::ErrorKind::NotFound, "no relative path for asset")
                        })?;

                self.old_relative_path = Some(old_relative);

                editor
                    .game
                    .asset_registry
                    .replace_record(key, AssetRecord::new(new_canonical))?;

                if let AssetKey::Prefab(_) = key {
                    Self::sync_prefab_rename(editor, key);
                }

                editor.save();
                Ok::<(), io::Error>(())
            })?;
        }

        Ok(())
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
                None,
            ) {
                push_toast(format!("Prefab save failed: {e}"), 3.0);
            }
        }
    }
}

impl EditorCommand for MoveFileCmd {
    fn execute(&mut self) {
        if let Err(e) = self.perform() {
            push_toast(format!("Move failed: {e}"), 3.0);
        }
    }

    fn undo(&mut self) {
        if let Err(e) = fs::rename(&self.new_full_path, &self.old_full_path) {
            push_toast(format!("Undo move failed: {e}"), 3.0);
            return;
        }

        if let Some(key) = self.key {
            if let Some(old_relative) = self.old_relative_path.take() {
                with_editor(|editor| {
                    let kind = AssetRegistry::kind_for_key(key);
                    let folder = AssetRegistry::asset_folder(kind);
                    let canonical = folder.join(&old_relative);

                    if let Err(e) = editor
                        .game
                        .asset_registry
                        .replace_record(key, AssetRecord::new(canonical))
                    {
                        push_toast(format!("Undo registry remap failed: {e}"), 3.0);
                        return;
                    }

                    if let AssetKey::Prefab(_) = key {
                        Self::sync_prefab_rename(editor, key);
                    }

                    editor.save();
                });
            }
        }
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
