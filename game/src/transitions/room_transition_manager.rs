use crate::engine::game_instance::GameInstance;
use bishop::prelude::Vec2;
use engine_core::ecs::*;
use engine_core::rendering::visual_position;
use engine_core::scripting::event_tags::event_tag::EventTag;
use engine_core::scripting::lua_constants::lua_events;
use engine_core::worlds::*;
use mlua::Lua;
use mlua::Value;
use mlua::Variadic;

const ROOM_PROBE_EPS: f32 = 0.01;

/// Observes ands handles room transitions through exits within the world.
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

            let pos = {
                let Some(transform) = game_instance.game.ecs.get::<Transform>(entity).copied() else {
                    continue;
                };
                let Some(collider) = game_instance.game.ecs.get::<Collider>(entity).copied() else {
                    continue;
                };
                room_probe_position(
                    transform,
                    collider,
                    game_instance.game.ecs.get::<SubPixel>(entity),
                )
            };
            let Some(target_id) = game_instance.game.current_world().room_at(pos) else {
                continue;
            };
            let Some(current_room) = game_instance
                .game
                .ecs
                .get::<CurrentRoom>(entity)
                .map(|room| room.0)
            else {
                continue;
            };
            if current_room == target_id {
                continue;
            }

            game_instance.game.ecs.set_current_room(entity, target_id);

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

/// Collects tags that should be emitted as part of a room-transition event.
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

fn collect_transition_tags(game_instance: &GameInstance, room_id: RoomId) -> Vec<EventTag> {
    game_instance
        .game
        .current_world()
        .get_room(room_id)
        .map(|room| room.tags.clone())
        .unwrap_or_default()
}
