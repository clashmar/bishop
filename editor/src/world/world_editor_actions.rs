// editor/src/world/world_editor_actions.rs
use crate::world::coord;
use crate::world::world_editor::WorldEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

impl WorldEditor {
    /// Delete a room by its RoomId.
    pub fn delete_room(&mut self, ctx: &mut GameCtxMut, room_id: RoomId) {
        let Some(cur_world) = ctx.cur_world.as_deref_mut() else {
            return;
        };

        // Find the index of the room we want to remove
        let idx = match cur_world.rooms.iter().position(|m| m.id == room_id) {
            Some(i) => i,
            None => return, // nothing to delete
        };

        // Remove the room from the world
        cur_world.rooms.remove(idx);

        // Re‑compute adjacency for the remaining rooms
        let len = cur_world.rooms.len();
        for i in 0..len {
            let (before, rest) = cur_world.rooms.split_at_mut(i);
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

        // Gather all entities from the current room.
        let mut entities_to_remove = Vec::new();
        {
            let current_room_store = ctx.ecs.get_store::<CurrentRoom>();
            for (&entity, &CurrentRoom(room)) in current_room_store.data.iter() {
                if room == room_id {
                    entities_to_remove.push(entity);
                }
            }
        }

        // Delete the entities
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
        };

        let id = game.allocate_room_id();
        let grid_size = game.current_world().grid_size;

        let room = Room {
            id,
            name: name.to_string(),
            position,
            size,
            exits: vec![],
            adjacent_rooms: vec![],
            variants: vec![variant],
            darkness: 0.,
        };

        room.create_room_camera(&mut game.ecs, id, grid_size);

        let cur_world = game.current_world_mut();
        cur_world.rooms.push(room);

        let len = cur_world.rooms.len();

        // Split the vector into "old rooms" and "the new room"
        let (old_slice, new_slice) = cur_world.rooms.split_at_mut(len - 1);
        let new_room = &mut new_slice[0];

        for old_room in old_slice.iter_mut() {
            if Self::are_rooms_adjacent(old_room, new_room) {
                old_room.adjacent_rooms.push(id);
                new_room.adjacent_rooms.push(old_room.id);
            }
        }

        id
    }

    fn are_rooms_adjacent(a: &Room, b: &Room) -> bool {
        let a_rect = Rect::new(a.position.x, a.position.y, a.size.x, a.size.y);
        let b_rect = Rect::new(b.position.x, b.position.y, b.size.x, b.size.y);

        // Rooms are adjacent if they share an edge
        let horizontal_touch = a_rect.x < b_rect.x + b_rect.w
            && a_rect.x + a_rect.w > b_rect.x
            && (a_rect.y + a_rect.h == b_rect.y || b_rect.y + b_rect.h == a_rect.y);

        let vertical_touch = a_rect.y < b_rect.y + b_rect.h
            && a_rect.y + a_rect.h > b_rect.y
            && (a_rect.x + a_rect.w == b_rect.x || b_rect.x + b_rect.w == a_rect.x);

        horizontal_touch || vertical_touch
    }

    /// Draws the coordinates of the grid square the mouse is over.
    pub fn draw_coordinates(&self, ctx: &mut WgpuContext, camera: &Camera2D, grid_size: f32) {
        let world_grid = coord::mouse_world_grid(ctx, camera, grid_size);

        let txt = format!("({:.0}, {:.0})", world_grid.x, world_grid.y,);

        let txt_metrics = measure_text(ctx, &txt, DEFAULT_FONT_SIZE_16);
        let margin = 10.0;

        let x = (ctx.screen_width() - txt_metrics.width) / 2.0;
        let y = ctx.screen_height() - margin;

        ctx.draw_text(&txt, x, y, DEFAULT_FONT_SIZE_16, Color::BLACK);
    }
}
