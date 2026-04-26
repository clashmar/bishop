// editor/src/commands/asset/remap_asset_path_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub struct RemapAssetPathCmd {
    key: AssetKey,
    new_relative_path: PathBuf,
    old_relative_path: Option<PathBuf>,
}

impl RemapAssetPathCmd {
    pub fn new(key: AssetKey, new_relative_path: impl Into<PathBuf>) -> Self {
        Self {
            key,
            new_relative_path: new_relative_path.into(),
            old_relative_path: None,
        }
    }
}

impl EditorCommand for RemapAssetPathCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let kind = AssetRegistry::kind_for_key(self.key);
            let old_relative = editor.game.asset_registry.relative_path(self.key);
            let folder = AssetRegistry::asset_folder(kind);
            let new_canonical = folder.join(&self.new_relative_path);

            if let Err(e) = editor
                .game
                .asset_registry
                .replace_record(self.key, AssetRecord::new(new_canonical))
            {
                push_toast(format!("Remap failed: {e}"), 3.0);
                return;
            }

            self.old_relative_path = old_relative;
            editor.save();
        });
    }

    fn undo(&mut self) {
        let Some(old_relative) = self.old_relative_path.take() else {
            return;
        };
        let key = self.key;

        with_editor(|editor| {
            let kind = AssetRegistry::kind_for_key(key);
            let folder = AssetRegistry::asset_folder(kind);
            let canonical = folder.join(&old_relative);

            if let Err(e) = editor
                .game
                .asset_registry
                .replace_record(key, AssetRecord::new(canonical))
            {
                push_toast(format!("Undo remap failed: {e}"), 3.0);
                return;
            }

            editor.save();
        });
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
