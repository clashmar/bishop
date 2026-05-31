use crate::engine::game_instance::GameInstance;
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_events;
use mlua::Lua;
use mlua::Value;
use mlua::Variadic;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransitionState {
    /// Normal state.
    #[default]
    None,
    /// Player has just crossed an exit boundary and still overlaps both rooms.
    Penetrated,
    /// Player is completely inside the target room.
    Entered,
    /// Player moved back into the previous room from overlapping state.
    Retreated,
}

#[derive(Default)]
pub struct TransitionManager {
    pub state: TransitionState,
    pub from: Option<Uuid>,
    pub to: Option<Uuid>,
}

impl TransitionManager {
    /// Called when the physics system reports that the player crossed an exit.
    pub fn set_state(&mut self, new_state: TransitionState, target_room: Uuid) {
        match new_state {
            TransitionState::Penetrated => {
                self.from = self.to;
                self.to = Some(target_room);
            }
            TransitionState::Entered => {
                self.state = TransitionState::None;
            }
            TransitionState::Retreated => {
                self.from = Some(target_room);
                self.to = None;
            }
            TransitionState::None => {}
        }
        self.state = new_state;
    }

    /// Helper to query if currently in a transition.
    pub fn in_transition(&self) -> bool {
        matches!(
            self.state,
            TransitionState::Penetrated | TransitionState::Retreated
        )
    }

    /// Handles entity transitions between rooms.
    pub fn handle_transitions(lua: &Lua, game_instance: &mut GameInstance) {
        let entities: Vec<_> = game_instance
            .game
            .ecs
            .get_store::<Transform>()
            .data
            .keys()
            .cloned()
            .collect();

        for entity in entities {
            let (pos, _coll) = {
                let p = match game_instance.game.ecs.get::<Transform>(entity) {
                    Some(v) => v.position,
                    None => continue,
                };
                let c = match game_instance.game.ecs.get::<Collider>(entity) {
                    Some(v) => v,
                    None => continue,
                };
                (p, c)
            };

            // Find the room that now contains the entity
            let target_id = match game_instance.game.current_world().room_at(pos) {
                Some(id) => id,
                None => continue,
            };

            if let Some(current_room) = game_instance
                .game
                .ecs
                .get::<CurrentRoom>(entity)
                .map(|room| room.0)
            {
                if current_room != target_id {
                    game_instance.game.ecs.set_current_room(entity, target_id);

                    if game_instance.game.ecs.get_player_entity() == Some(entity) {
                        if let Some(world) = game_instance.game.current_world_mut() {
                            world.current_room_id = Some(target_id);
                        }
                        let room_tags = collect_transition_tags(game_instance, target_id);

                        let mut args = vec![Value::Integer(target_id.0 as i64)];
                        for tag in room_tags {
                            if let Ok(lua_tag) = lua.create_string(&tag) {
                                args.push(Value::String(lua_tag));
                            }
                        }

                        game_instance.game.script_manager.event_bus.emit(
                            lua_events::ROOM_ENTERED.to_string(),
                            Variadic::from_iter(args),
                        );
                    }
                }
            }
        }
    }
}

/// Collects tags that should be emitted as part of a room-transition event.
fn collect_transition_tags(game_instance: &GameInstance, room_id: RoomId) -> Vec<String> {
    game_instance
        .game
        .current_world()
        .get_room(room_id)
        .map(|room| room.tags.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::scripting::event_bus::EventBus;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn make_two_rooms() -> (Room, Room) {
        let a = Room { id: RoomId(1), position: Vec2::ZERO, size: Vec2::new(32.0, 32.0), ..Default::default() };
        let b = Room { id: RoomId(2), position: Vec2::new(32.0, 0.0), size: Vec2::new(32.0, 32.0), ..Default::default() };
        (a, b)
    }

    fn setup_tagged_game(tags: Vec<String>) -> (Lua, Game, Entity, Arc<Mutex<Vec<String>>>) {
        let lua = Lua::new();
        let event_bus = EventBus::default();
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let (room_a, mut room_b) = make_two_rooms();
        room_b.tags = tags;
        let mut world = World::default();
        world.add_room(room_a);
        world.add_room(room_b);
        world.grid_size = 1.0;
        world.rebuild_room_grid();
        let mut game = Game::default();
        game.script_manager.event_bus = event_bus.clone();
        game.add_world(world);
        let player = game.ecs.create_entity()
            .with(Transform { position: Vec2::new(40.0, 8.0), ..Default::default() })
            .with(Collider::default())
            .with(Player::default())
            .with_current_room(RoomId(1))
            .finish();
        let capt = received.clone();
        let handler = lua.create_function(move |_lua, args: Variadic<Value>| {
            let mut vals = capt.lock().unwrap();
            vals.clear();
            for arg in args {
                match arg {
                    Value::Integer(n) => vals.push(n.to_string()),
                    Value::String(s) => vals.push(s.to_str().unwrap().to_string()),
                    other => panic!("unexpected value: {other:?}"),
                }
            }
            Ok(())
        }).unwrap();
        event_bus.on(lua_events::ROOM_ENTERED.to_string(), handler);
        (lua, game, player, received)
    }

    #[test]
    fn handle_transitions_moves_membership_between_rooms() {
        let (room_a, room_b) = make_two_rooms();

        let mut world = World::default();
        world.add_room(room_a);
        world.add_room(room_b);
        world.grid_size = 1.0;
        world.rebuild_room_grid();

        let mut game = Game::default();
        game.add_world(world);
        let entity = game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(40.0, 8.0),
                ..Default::default()
            })
            .with(Collider::default())
            .with_current_room(RoomId(1))
            .finish();

        let mut game_instance = GameInstance {
            game,
            prev_positions: HashMap::new(),
        };

        let lua = Lua::new();
        TransitionManager::handle_transitions(&lua, &mut game_instance);

        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<CurrentRoom>(entity)
                .map(|room| room.0),
            Some(RoomId(2))
        );
        let room_b_entities = game_instance.game.ecs.entities_in_room(RoomId(2));
        assert!(room_b_entities.contains(&entity));
        let room_a_entities = game_instance.game.ecs.entities_in_room(RoomId(1));
        assert!(!room_a_entities.contains(&entity));
    }

    #[test]
    fn room_entered_event_carries_tags_when_room_has_tags() {
        let (lua, game, player, received) = setup_tagged_game(vec!["autosave".into()]);
        let mut game_instance = GameInstance { game, prev_positions: HashMap::new() };
        TransitionManager::handle_transitions(&lua, &mut game_instance);
        assert_eq!(game_instance.game.ecs.get_player_entity(), Some(player));
        assert_eq!(received.lock().unwrap().as_slice(), ["2", "autosave"]);
    }

    #[test]
    fn room_entered_event_omits_extra_args_when_room_has_no_tags() {
        let (lua, game, _player, received) = setup_tagged_game(vec![]);
        let mut game_instance = GameInstance { game, prev_positions: HashMap::new() };
        TransitionManager::handle_transitions(&lua, &mut game_instance);
        assert_eq!(received.lock().unwrap().as_slice(), ["2"]);
    }
}

