use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::tiles::{TileDef, TileDefId};

/// Undo-able command for deleting a tile definition.
#[derive(Debug)]
pub struct DeleteTileDefinitionCmd {
    tile_id: TileDefId,
    deleted_def: Option<TileDef>,
}

impl DeleteTileDefinitionCmd {
    pub fn new(tile_id: TileDefId) -> Self {
        Self {
            tile_id,
            deleted_def: None,
        }
    }
}

impl EditorCommand for DeleteTileDefinitionCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let removed = editor.game.tile_registry.remove(self.tile_id);
            if self.deleted_def.is_none() {
                self.deleted_def = removed;
            }
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            if let Some(tile_def) = self.deleted_def.clone() {
                editor.game.tile_registry.replace(self.tile_id, tile_def);
            }
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        matches!(current_mode, EditorMode::Room(_))
    }
}
