use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::tiles::{TileDef, TileDefId};

/// Undo-able command for updating a tile definition.
#[derive(Debug)]
pub struct UpdateTileDefinitionCmd {
    tile_id: TileDefId,
    before: TileDef,
    after: TileDef,
}

impl UpdateTileDefinitionCmd {
    pub fn new(tile_id: TileDefId, before: TileDef, after: TileDef) -> Self {
        Self {
            tile_id,
            before,
            after,
        }
    }

    fn apply(tile_id: TileDefId, tile_def: TileDef) {
        with_editor(|editor| {
            editor.game.tile_registry.replace(tile_id, tile_def);
            editor.game.sync_tile_definition(tile_id);
        });
    }
}

impl EditorCommand for UpdateTileDefinitionCmd {
    fn execute(&mut self) {
        Self::apply(self.tile_id, self.after.clone());
    }

    fn undo(&mut self) {
        Self::apply(self.tile_id, self.before.clone());
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        matches!(current_mode, EditorMode::Room(_))
    }
}
