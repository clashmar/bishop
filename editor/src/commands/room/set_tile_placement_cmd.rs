use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::ecs::{Ecs, TilePlacement};
use engine_core::tiles::{TileDefId, apply_tile_placement_definition};
use engine_core::worlds::{RoomId, RoomLayer};

#[derive(Debug)]
pub struct SetTilePlacementCmd {
    room_id: RoomId,
    layer: RoomLayer,
    cell: (usize, usize),
    before: Option<TilePlacement>,
    after: Option<TilePlacement>,
    state_captured: bool,
}

impl SetTilePlacementCmd {
    pub fn place(room_id: RoomId, layer: RoomLayer, cell: (usize, usize), definition: TileDefId) -> Self {
        Self {
            room_id,
            layer,
            cell,
            before: None,
            after: Some(TilePlacement::new(definition, cell.0, cell.1)),
            state_captured: false,
        }
    }

    pub fn clear(room_id: RoomId, layer: RoomLayer, cell: (usize, usize)) -> Self {
        Self {
            room_id,
            layer,
            cell,
            before: None,
            after: None,
            state_captured: false,
        }
    }

    fn capture_state(&mut self) {
        if self.state_captured {
            return;
        }

        with_editor(|editor| {
            self.before = editor
                .game
                .ecs
                .tile_placement_at(self.room_id, self.layer, self.cell.0, self.cell.1);
        });

        self.state_captured = true;
    }

    fn apply(&self, placement: Option<TilePlacement>) {
        with_editor(|editor| {
            let existing_entity = editor
                .game
                .ecs
                .tile_entity_at(self.room_id, self.layer, self.cell.0, self.cell.1);

            let ctx = &mut editor.game.ctx_mut();
            if let Some(entity) = existing_entity {
                Ecs::remove_entity(ctx, entity);
            }

            if let Some(placement) = placement {
                let entity = ctx
                    .ecs
                    .create_entity()
                    .with(placement)
                    .with_current_room_layer(self.room_id, self.layer)
                    .finish();
                apply_tile_placement_definition(ctx, entity);
            }
        });
    }
}

impl EditorCommand for SetTilePlacementCmd {
    fn execute(&mut self) {
        self.capture_state();
        self.apply(self.after);
    }

    fn undo(&mut self) {
        self.apply(self.before);
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Room(self.room_id)
    }
}
