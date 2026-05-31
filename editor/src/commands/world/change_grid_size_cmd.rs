use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for changing a world's grid size.
#[derive(Debug)]
pub struct ChangeGridSizeCmd {
    world_id: WorldId,
    old_grid_size: f32,
    new_grid_size: f32,
    old_room_positions: Vec<(RoomId, Vec2)>,
    old_entity_positions: Vec<(Entity, Vec2)>,
}

impl ChangeGridSizeCmd {
    pub fn new(world_id: WorldId, old_grid_size: f32, new_grid_size: f32) -> Self {
        Self {
            world_id,
            old_grid_size,
            new_grid_size,
            old_room_positions: Vec::new(),
            old_entity_positions: Vec::new(),
        }
    }
}

impl EditorCommand for ChangeGridSizeCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let world = match editor.game.get_world_mut(self.world_id) {
                Some(w) => w,
                None => return,
            };

            if (self.new_grid_size - self.old_grid_size).abs() < 0.001 {
                return;
            }

            // Capture room positions before scaling
            self.old_room_positions = world.rooms().iter().map(|r| (r.id, r.position)).collect();

            // Capture entity positions before scaling
            let trans_store = editor.game.ecs.get_store::<Transform>();
            self.old_entity_positions = trans_store
                .data
                .iter()
                .map(|(&entity, t)| (entity, t.position))
                .collect();

            let scale_factor = self.new_grid_size / self.old_grid_size;

            // Set the new grid size
            let world = match editor.game.get_world_mut(self.world_id) {
                Some(w) => w,
                None => return,
            };
            world.grid_size = self.new_grid_size;

            // Scale room positions
            for room in world.rooms_mut() {
                room.position *= scale_factor;
            }

            world.rebuild_room_grid();

            // Scale entity positions
            let pos_store = editor.game.ecs.get_store_mut::<Transform>();
            for transform in pos_store.data.values_mut() {
                transform.position *= scale_factor;
            }

            push_toast(
                format!("World grid size changed to {}", self.new_grid_size),
                2.5,
            );
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let world = match editor.game.get_world_mut(self.world_id) {
                Some(w) => w,
                None => return,
            };

            // Restore grid size
            world.grid_size = self.old_grid_size;

            // Restore exact room positions
            for (room_id, position) in &self.old_room_positions {
                if let Some(room) = world.rooms_mut().iter_mut().find(|r| r.id == *room_id) {
                    room.position = *position;
                }
            }

            world.rebuild_room_grid();

            // Restore exact entity positions
            let pos_store = editor.game.ecs.get_store_mut::<Transform>();
            for (entity, position) in &self.old_entity_positions {
                if let Some(transform) = pos_store.data.get_mut(entity) {
                    transform.position = *position;
                }
            }

            push_toast(
                format!("World grid size restored to {}", self.old_grid_size),
                2.5,
            );
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        match current_mode {
            EditorMode::World(id) => id == self.world_id,
            EditorMode::Room(room_id) => with_editor(|editor| {
                editor
                    .game
                    .worlds()
                    .iter()
                    .find(|w| w.id == self.world_id)
                    .and_then(|w| w.get_room(room_id))
                    .is_some()
            }),
            _ => false,
        }
    }
}
