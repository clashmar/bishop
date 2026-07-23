use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::tiles::{TileDef, TileDefId};

/// Undo-able command for creating a tile definition.
#[derive(Debug)]
pub struct CreateTileDefinitionCmd {
    tile_def: TileDef,
    created_id: Option<TileDefId>,
}

impl CreateTileDefinitionCmd {
    pub fn new(tile_def: TileDef) -> Self {
        Self {
            tile_def,
            created_id: None,
        }
    }
}

impl EditorCommand for CreateTileDefinitionCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            if let Some(tile_id) = self.created_id {
                editor.game.tile_registry.replace(tile_id, self.tile_def.clone());
            } else {
                self.created_id = Some(editor.game.tile_registry.insert(self.tile_def.clone()));
            }
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            if let Some(tile_id) = self.created_id {
                editor.game.tile_registry.remove(tile_id);
            }
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        matches!(current_mode, EditorMode::Room(_))
    }
}
