use crate::app::EditorMode;
use crate::commands::asset::register_asset_cmd::register_asset;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::assets::{AssetKey, AssetKind, AssetRegistry};
use engine_core::storage::path_utils::resources_folder_current;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ImportAssetCmd {
    kind: AssetKind,
    source_path: PathBuf,
    destination_directory: Option<PathBuf>,
    registered_key: Option<AssetKey>,
    imported_path: Option<PathBuf>,
    newly_registered: bool,
}

impl ImportAssetCmd {
    pub fn new(kind: AssetKind, source_path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            source_path: source_path.into(),
            destination_directory: None,
            registered_key: None,
            imported_path: None,
            newly_registered: false,
        }
    }

    pub fn new_in_directory(
        kind: AssetKind,
        source_path: impl Into<PathBuf>,
        destination_directory: impl Into<PathBuf>,
    ) -> Self {
        let mut command = Self::new(kind, source_path);
        command.destination_directory = Some(destination_directory.into());
        command
    }

    fn import(&mut self) -> io::Result<()> {
        let kind = self.kind;
        let source_path = &self.source_path;
        let (key, newly_registered, imported_path) = with_editor(|editor| {
            if !source_path.is_file() {
                return Err(io::Error::new(ErrorKind::NotFound, "source file does not exist"));
            }
            let file_name = source_path.file_name().ok_or_else(|| {
                io::Error::new(ErrorKind::InvalidInput, "source file has no filename")
            })?;
            let destination_directory = self
                .destination_directory
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    resources_folder_current().join(AssetRegistry::asset_folder(kind))
                });
            let target = destination_directory.join(file_name);

            if source_path == &target {
                let (key, newly_registered) = register_asset(editor, kind, &target)?;
                return Ok((key, newly_registered, None));
            }
            if target.exists() {
                if editor.game.asset_registry.key_for_full_path(&target).is_some() {
                    let (key, newly_registered) = register_asset(editor, kind, &target)?;
                    return Ok((key, newly_registered, None));
                }
                return Err(io::Error::new(
                    ErrorKind::AlreadyExists,
                    format!("'{}' already exists", target.display()),
                ));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &target)?;
            match register_asset(editor, kind, &target) {
                Ok((key, newly_registered)) => Ok((key, newly_registered, Some(target))),
                Err(error) => {
                    let _ = fs::remove_file(&target);
                    Err(error)
                }
            }
        })?;
        self.registered_key = Some(key);
        self.newly_registered = newly_registered;
        self.imported_path = imported_path;
        Ok(())
    }
}

impl EditorCommand for ImportAssetCmd {
    fn execute(&mut self) {
        if let Err(error) = self.import() {
            push_toast(format!("Import failed: {error}"), 3.0);
        }
    }

    fn undo(&mut self) {
        if !self.newly_registered && self.imported_path.is_none() {
            return;
        }
        let key = self.registered_key.take();
        let imported_path = self.imported_path.take();
        with_editor(|editor| {
            if self.newly_registered {
                if let Some(key) = key {
                    editor.game.asset_registry.remove_record(key);
                    if matches!(key, AssetKey::Prefab(_)) {
                        editor.game.reload_prefab_manager();
                    }
                }
            }
            if let Some(path) = imported_path {
                if let Err(error) = fs::remove_file(path) {
                    push_toast(format!("Undo import failed: {error}"), 3.0);
                    return;
                }
            }
            editor.save();
        });
        self.newly_registered = false;
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
