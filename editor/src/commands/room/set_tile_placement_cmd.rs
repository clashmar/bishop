use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::ecs::{Ecs, Entity, TilePlacement};
use engine_core::game::GameCtxMut;
use engine_core::tiles::{TileDefId, apply_tile_placement_definition};
use engine_core::worlds::RoomId;

#[derive(Debug)]
pub struct SetTilePlacementCmd {
    room_id: RoomId,
    cell: (usize, usize),
    before: Option<TilePlacement>,
    after: Option<TilePlacement>,
    state_captured: bool,
}

impl SetTilePlacementCmd {
    pub fn place(room_id: RoomId, cell: (usize, usize), definition: TileDefId) -> Self {
        Self {
            room_id,
            cell,
            before: None,
            after: Some(TilePlacement::new(definition, cell.0, cell.1)),
            state_captured: false,
        }
    }

    pub fn clear(room_id: RoomId, cell: (usize, usize)) -> Self {
        Self {
            room_id,
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
            self.before = room_tile_placements(&editor.game.ecs, self.room_id)
                .into_iter()
                .find(|placement| (placement.grid_x, placement.grid_y) == self.cell);
        });

        self.state_captured = true;
    }

    fn apply(&self, placement: Option<TilePlacement>) {
        with_editor(|editor| {
            let existing_entities: Vec<_> = editor
                .game
                .ecs
                .entities_in_room(self.room_id)
                .iter()
                .copied()
                .filter(|entity| {
                    editor
                        .game
                        .ecs
                        .get::<TilePlacement>(*entity)
                        .is_some_and(|tile| (tile.grid_x, tile.grid_y) == self.cell)
                })
                .collect();

            let ctx = &mut editor.game.ctx_mut();
            remove_entities(ctx, &existing_entities);

            if let Some(placement) = placement {
                let entity = ctx
                    .ecs
                    .create_entity()
                    .with(placement)
                    .with_current_room(self.room_id)
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

fn room_tile_placements(ecs: &Ecs, room_id: RoomId) -> Vec<TilePlacement> {
    ecs.entities_in_room(room_id)
        .iter()
        .copied()
        .filter_map(|entity| ecs.get::<TilePlacement>(entity).copied())
        .collect()
}

fn remove_entities(ctx: &mut GameCtxMut<'_>, entities: &[Entity]) {
    for &entity in entities {
        Ecs::remove_entity(ctx, entity);
    }
}
