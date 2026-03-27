// engine_core/src/world/transition_manager.rs
use crate::engine::game_instance::GameInstance;
use engine_core::prelude::*;
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
    pub fn handle_transitions(game_instance: &mut GameInstance) {
        let grid_size = game_instance.game.current_world().grid_size;
        let rooms = game_instance.game.current_world().rooms.clone();

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
            let target_id = match room_of_entity(pos, &rooms, grid_size) {
                Some(id) => id,
                None => continue,
            };

            if let Some(comp) = game_instance.game.ecs.get_mut::<CurrentRoom>(entity) {
                if comp.0 == target_id {
                    continue;
                } else {
                    comp.0 = target_id
                }
            }

            if game_instance.game.ecs.get_player_entity() == Some(entity) {
                if let Some(new_room) = rooms.iter().find(|r| r.id == target_id) {
                    game_instance.game.current_world_mut().current_room_id = Some(new_room.id);
                }
            }
        }
    }
}

/// Return the id of the room whose bounds contain the entity's AABB.
pub fn room_of_entity(pos: Vec2, rooms: &[Room], grid_size: f32) -> Option<RoomId> {
    for room in rooms {
        let min = room.position;
        let max = room.position + room.size * grid_size;

        // Never use <=/>= here or will overlap with adjacent rooms
        if pos.x >= min.x && pos.x < max.x && pos.y > min.y && pos.y <= max.y {
            return Some(room.id);
        }
    }
    None
}
