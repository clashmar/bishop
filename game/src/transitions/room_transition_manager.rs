use crate::engine::game_instance::GameInstance;
use bishop::prelude::Vec2;
use engine_core::ecs::{Collider, CurrentRoom, SubPixel, Transform, pivot_offset};
use engine_core::rendering::visual_position;
use engine_core::scripting::event_tags::event_tag::EventTag;
use engine_core::scripting::lua_constants::lua_events;
use engine_core::worlds::{ExitDirection, Room, RoomId, RoomLayer};
use mlua::Lua;
use mlua::Value;
use mlua::Variadic;

const ROOM_PROBE_EPS: f32 = 0.01;

/// Observes and handles room transitions through exits within the world.
pub struct RoomTransitionManager;

impl RoomTransitionManager {
    /// Handles entity transitions between rooms.
    pub fn handle_transitions(lua: &Lua, game_instance: &mut GameInstance) -> bool {
        let entities: Vec<_> = game_instance
            .game
            .ecs
            .get_store::<Transform>()
            .data
            .keys()
            .copied()
            .collect();

        let mut player_transitioned = false;

        for entity in entities {
            if !game_instance.game.entity_in_active_world(entity) {
                continue;
            }

            let Some(transform) = game_instance.game.ecs.get::<Transform>(entity).copied() else {
                continue;
            };
            let Some(collider) = game_instance.game.ecs.get::<Collider>(entity).copied() else {
                continue;
            };
            let sub_pixel = game_instance.game.ecs.get::<SubPixel>(entity);
            let pos = room_probe_position(transform, collider, sub_pixel);
            let bounds = room_probe_bounds(transform, collider, sub_pixel);
            let Some(target_id) = game_instance.game.current_world().room_at(pos) else {
                continue;
            };
            let Some(current_room) = game_instance.game.ecs.get::<CurrentRoom>(entity).copied() else {
                continue;
            };
            if current_room.room_id == target_id {
                continue;
            }

            let world = game_instance.game.current_world();
            let can_transition = world.get_room(current_room.room_id).is_some_and(|room| {
                room_has_layer_exit_to(room, target_id, current_room.layer, bounds, world.grid_size)
            });
            if !can_transition {
                continue;
            }

            game_instance
                .game
                .ecs
                .set_current_room_layer(entity, target_id, current_room.layer);

            if game_instance.game.ecs.get_player_entity() == Some(entity) {
                if let Some(world) = game_instance.game.current_world_mut() {
                    world.current_room_id = Some(target_id);
                }
                let room_tags = collect_transition_tags(game_instance, target_id);

                let mut args = vec![Value::Integer(target_id.0 as i64)];
                for tag in room_tags {
                    if let Ok(lua_tag) = lua.create_string(tag.lua_name()) {
                        args.push(Value::String(lua_tag));
                    }
                }

                game_instance.game.script_manager.event_bus.emit(
                    lua_events::ROOM_ENTERED.to_string(),
                    Variadic::from_iter(args),
                );

                player_transitioned = true;
            }
        }

        player_transitioned
    }
}

/// Returns the inward-biased probe position used to detect room membership.
fn room_probe_position(transform: Transform, collider: Collider, sub_pixel: Option<&SubPixel>) -> Vec2 {
    let pos = visual_position(transform.position, sub_pixel) + collider.offset;
    let (sw, sh) = collider.shape.size();
    let size = Vec2::new(sw, sh);
    let top_left = pivot_offset(pos, size, transform.pivot);
    let to_center = (top_left + size * 0.5) - pos;
    let inward = if to_center.length_squared() > 0.0 {
        to_center.normalize() * ROOM_PROBE_EPS
    } else {
        Vec2::ZERO
    };
    pos + inward
}

fn room_probe_bounds(transform: Transform, collider: Collider, sub_pixel: Option<&SubPixel>) -> (Vec2, Vec2) {
    let pos = visual_position(transform.position, sub_pixel) + collider.offset;
    let (sw, sh) = collider.shape.size();
    let size = Vec2::new(sw, sh);
    let top_left = pivot_offset(pos, size, transform.pivot);
    (top_left, top_left + size)
}

/// Collects tags that should be emitted as part of a room-transition event.
fn collect_transition_tags(game_instance: &GameInstance, room_id: RoomId) -> Vec<EventTag> {
    game_instance
        .game
        .current_world()
        .get_room(room_id)
        .map(|room| room.tags.clone())
        .unwrap_or_default()
}

fn room_has_layer_exit_to(
    room: &Room,
    target_room_id: RoomId,
    layer: RoomLayer,
    bounds: (Vec2, Vec2),
    grid_size: f32,
) -> bool {
    room.exits.iter().any(|exit| {
        exit.target_room_id == Some(target_room_id)
            && exit.layer == layer
            && bounds_overlap_exit(room, exit.direction, exit.position, bounds, grid_size)
    })
}

fn bounds_overlap_exit(
    room: &Room,
    direction: ExitDirection,
    exit_position: Vec2,
    bounds: (Vec2, Vec2),
    grid_size: f32,
) -> bool {
    let (min, max) = bounds;
    let room_min = room.position;
    let room_max = room.position + room.size * grid_size;

    match direction {
        ExitDirection::Right => {
            let exit_min = room.position.y + exit_position.y * grid_size;
            let exit_max = exit_min + grid_size;
            min.x <= room_max.x + ROOM_PROBE_EPS
                && max.x > room_max.x - ROOM_PROBE_EPS
                && ranges_overlap(min.y, max.y, exit_min, exit_max)
        }
        ExitDirection::Left => {
            let exit_min = room.position.y + exit_position.y * grid_size;
            let exit_max = exit_min + grid_size;
            min.x < room_min.x + ROOM_PROBE_EPS
                && max.x >= room_min.x - ROOM_PROBE_EPS
                && ranges_overlap(min.y, max.y, exit_min, exit_max)
        }
        ExitDirection::Down => {
            let exit_min = room.position.x + exit_position.x * grid_size;
            let exit_max = exit_min + grid_size;
            min.y <= room_max.y + ROOM_PROBE_EPS
                && max.y > room_max.y - ROOM_PROBE_EPS
                && ranges_overlap(min.x, max.x, exit_min, exit_max)
        }
        ExitDirection::Up => {
            let exit_min = room.position.x + exit_position.x * grid_size;
            let exit_max = exit_min + grid_size;
            min.y < room_min.y + ROOM_PROBE_EPS
                && max.y >= room_min.y - ROOM_PROBE_EPS
                && ranges_overlap(min.x, max.x, exit_min, exit_max)
        }
    }
}

fn ranges_overlap(min_a: f32, max_a: f32, min_b: f32, max_b: f32) -> bool {
    min_a < max_b - ROOM_PROBE_EPS && max_a > min_b + ROOM_PROBE_EPS
}
