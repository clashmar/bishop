use bishop::prelude::*;
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use engine_core::ecs::*;
use engine_core::worlds::*;

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
            if (self.new_grid_size - self.old_grid_size).abs() < 0.001 {
                return;
            }

            let room_ids: Vec<RoomId> = {
                let world = match editor.game.get_world(self.world_id) {
                    Some(w) => w,
                    None => return,
                };
                self.old_room_positions = world.rooms().iter().map(|r| (r.id, r.position)).collect();
                world.rooms().iter().map(|room| room.id).collect()
            };

            // Capture entity positions before scaling
            let trans_store = editor.game.ecs.get_store::<Transform>();
            self.old_entity_positions = trans_store
                .data
                .iter()
                .filter(|(entity, _)| roomed_entity_in_rooms(&editor.game.ecs, &room_ids, **entity))
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

            let entities_to_scale: Vec<Entity> = {
                let trans_store = editor.game.ecs.get_store::<Transform>();
                trans_store
                    .data
                    .keys()
                    .copied()
                    .filter(|entity| roomed_entity_in_rooms(&editor.game.ecs, &room_ids, *entity))
                    .collect()
            };

            // Scale entity positions
            let pos_store = editor.game.ecs.get_store_mut::<Transform>();
            for entity in entities_to_scale {
                if let Some(transform) = pos_store.data.get_mut(&entity) {
                    transform.position *= scale_factor;
                }
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

fn roomed_entity_in_rooms(ecs: &Ecs, room_ids: &[RoomId], entity: Entity) -> bool {
    ecs.get::<CurrentRoom>(entity)
        .is_some_and(|room| room_ids.contains(&room.room_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Editor;
    use crate::test_utils::EditorServicesGuard;
    use engine_core::game::Game;

    fn install_editor_with_two_world_entities() -> (EditorServicesGuard, WorldId, RoomId, Entity, WorldId, RoomId, Entity) {
        let mut game = Game::default();

        let world_a_id = WorldId(1);
        let room_a_id = RoomId(1);
        let mut world_a = World::new(world_a_id, "World A".to_string(), 16.0);
        let mut room_a = Room::new(&mut game.ecs, room_a_id, 16.0);
        room_a.position = Vec2::new(32.0, 48.0);
        world_a.add_room(room_a);
        game.add_world(world_a);

        let world_b_id = WorldId(2);
        let room_b_id = RoomId(2);
        let mut world_b = World::new(world_b_id, "World B".to_string(), 16.0);
        let mut room_b = Room::new(&mut game.ecs, room_b_id, 16.0);
        room_b.position = Vec2::new(128.0, 160.0);
        world_b.add_room(room_b);
        game.add_world(world_b);

        game.select_world(world_a_id);

        let world_a_entity = game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(64.0, 80.0),
                ..Default::default()
            })
            .with_current_room(room_a_id)
            .finish();
        let world_b_entity = game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(96.0, 112.0),
                ..Default::default()
            })
            .with_current_room(room_b_id)
            .finish();

        let editor = Editor {
            game,
            mode: EditorMode::World(world_a_id),
            cur_world_id: Some(world_a_id),
            cur_room_id: Some(room_a_id),
            ..Default::default()
        };

        let guard = EditorServicesGuard::install(editor);
        (guard, world_a_id, room_a_id, world_a_entity, world_b_id, room_b_id, world_b_entity)
    }

    #[test]
    fn change_grid_size_cmd_when_world_changes_then_other_world_entities_keep_positions() {
        let (_guard, world_a_id, room_a_id, world_a_entity, world_b_id, room_b_id, world_b_entity) =
            install_editor_with_two_world_entities();
        let mut cmd = ChangeGridSizeCmd::new(world_a_id, 16.0, 32.0);

        cmd.execute();

        with_editor(|editor| {
            assert_eq!(editor.game.get_world(world_a_id).map(|world| world.grid_size), Some(32.0));
            assert_eq!(
                editor
                    .game
                    .get_world(world_a_id)
                    .and_then(|world| world.get_room(room_a_id))
                    .map(|room| room.position),
                Some(Vec2::new(64.0, 96.0))
            );
            assert_eq!(
                editor
                    .game
                    .ecs
                    .get::<Transform>(world_a_entity)
                    .map(|transform| transform.position),
                Some(Vec2::new(128.0, 160.0))
            );
            assert_eq!(
                editor
                    .game
                    .get_world(world_b_id)
                    .and_then(|world| world.get_room(room_b_id))
                    .map(|room| room.position),
                Some(Vec2::new(128.0, 160.0))
            );
            assert_eq!(
                editor
                    .game
                    .ecs
                    .get::<Transform>(world_b_entity)
                    .map(|transform| transform.position),
                Some(Vec2::new(96.0, 112.0))
            );
        });
    }
}
