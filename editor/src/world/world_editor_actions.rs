use crate::world::world_editor::WorldEditor;
use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::game::{Game, GameCtxMut};
use engine_core::tiles::TileMap;
use engine_core::worlds::*;
use std::collections::HashSet;

impl WorldEditor {
    /// Delete a room by its RoomId.
    pub fn delete_room(&mut self, ctx: &mut GameCtxMut, room_id: RoomId) {
        let Some(world) = ctx.world.as_deref_mut() else {
            return;
        };

        let Some(removed) = world.remove_room(room_id) else {
            return;
        };

        // Re‑compute adjacency for the remaining rooms
        let len = world.rooms().len();
        let rooms = world.rooms_mut();
        for i in 0..len {
            let (before, rest) = rooms.split_at_mut(i);
            let (room_i, after) = rest.split_first_mut().unwrap();
            room_i.adjacent_rooms.clear();

            for other in before.iter() {
                if Self::are_rooms_adjacent(room_i, other) {
                    room_i.adjacent_rooms.push(other.id);
                }
            }
            for other in after.iter() {
                if Self::are_rooms_adjacent(room_i, other) {
                    room_i.adjacent_rooms.push(other.id);
                }
            }
        }

        world.rebuild_room_grid();

        let mut entities_to_remove: HashSet<Entity> =
            ctx.ecs.entities_in_room(room_id).iter().copied().collect();
        entities_to_remove.insert(removed.singleton);

        for entity in entities_to_remove {
            Ecs::remove_entity(ctx, entity);
        }
    }

    /// Helper used by the UI when the user finishes a drag‑to‑place.
    /// Places a room in the current world.
    pub fn place_room_from_drag(
        &mut self,
        game: &mut Game,
        top_left: Vec2,
        size: Vec2,
        grid_size: f32,
    ) -> RoomId {
        let origin_in_pixels = top_left * grid_size;
        self.create_new_room(game, "untitled", origin_in_pixels, size)
    }

    /// Create a new room in the current world and return its id.
    pub fn create_new_room(
        &mut self,
        game: &mut Game,
        name: &str,
        position: Vec2,
        size: Vec2,
    ) -> RoomId {
        let tilemap = TileMap::new(size.x as usize, size.y as usize);

        let variant = RoomVariant {
            id: "default".to_string(),
            tilemap,
            ..Default::default()
        };

        let id = game.id_allocator.allocate_room_id();
        let grid_size = game.current_world().grid_size;

        let room = Room {
            id,
            name: name.to_string(),
            position,
            size,
            exits: vec![],
            adjacent_rooms: vec![],
            tags: vec![],
            variants: vec![variant],
            darkness: 0.,
            singleton: Room::create_room_singleton_entity(&mut game.ecs, id),
        };

        Room::create_camera_entity(&mut game.ecs, room.id, room.position, grid_size);

        let cur_world = game
            .current_world_mut()
            .expect("add_room requires a current world");

        cur_world.add_room(room);

        let len = cur_world.rooms().len();

        // Split the vector into "old rooms" and "the new room"
        let rooms = cur_world.rooms_mut();
        let (old_slice, new_slice) = rooms.split_at_mut(len - 1);
        let new_room = &mut new_slice[0];

        for old_room in old_slice.iter_mut() {
            if Self::are_rooms_adjacent(old_room, new_room) {
                old_room.adjacent_rooms.push(id);
                new_room.adjacent_rooms.push(old_room.id);
            }
        }

        cur_world.rebuild_room_grid();

        id
    }

    fn are_rooms_adjacent(a: &Room, b: &Room) -> bool {
        let a_rect = Rect::new(a.position.x, a.position.y, a.size.x, a.size.y);
        let b_rect = Rect::new(b.position.x, b.position.y, b.size.x, b.size.y);

        let horizontal_touch = a_rect.x < b_rect.x + b_rect.w
            && a_rect.x + a_rect.w > b_rect.x
            && (a_rect.y + a_rect.h == b_rect.y || b_rect.y + b_rect.h == a_rect.y);

        let vertical_touch = a_rect.y < b_rect.y + b_rect.h
            && a_rect.y + a_rect.h > b_rect.y
            && (a_rect.x + a_rect.w == b_rect.x || b_rect.x + b_rect.w == a_rect.x);

        horizontal_touch || vertical_touch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::{CurrentRoom, Singleton};

    #[test]
    fn create_new_room_when_called_assigns_room_singleton() {
        let mut game = Game::default();
        game.add_world(World::new(WorldId(1), "test".to_string(), 16.0));
        let mut editor = WorldEditor::new();

        let room_id = editor.create_new_room(
            &mut game,
            "Room",
            Vec2::new(32.0, 48.0),
            Vec2::new(4.0, 3.0),
        );

        let room = game.current_world().get_room(room_id).unwrap();
        let singleton = room.singleton;

        assert!(game.ecs.has::<Singleton>(singleton));
        assert_eq!(
            game.ecs.get::<CurrentRoom>(singleton).map(|current| current.room_id),
            Some(room_id)
        );
    }

    #[test]
    fn delete_room_when_called_removes_room_singleton() {
        let mut game = Game::default();
        game.add_world(World::new(WorldId(1), "test".to_string(), 16.0));
        let mut editor = WorldEditor::new();
        let room_id = editor.create_new_room(
            &mut game,
            "Room",
            Vec2::new(32.0, 48.0),
            Vec2::new(4.0, 3.0),
        );
        let singleton = game.current_world().get_room(room_id).unwrap().singleton;

        {
            let mut ctx = game.ctx_mut();
            editor.delete_room(&mut ctx, room_id);
        }

        assert!(game.current_world().get_room(room_id).is_none());
        assert!(!game.ecs.has::<Singleton>(singleton));
    }
}
