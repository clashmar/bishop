use crate::engine::game_instance::GameInstance;
use crate::game_global::set_pending_world_transition;
use crate::transitions::world_transitions::{WorldSelector, WorldTransitionRequest};
use engine_core::ecs::*;
use engine_core::worlds::world::{WorldExitTrigger, WorldTransitionMode};

/// Per-frame system that fires `WorldExit { OnProximity }` components when the player is in range.
pub struct WorldExitManager;

impl WorldExitManager {
    /// Fires proximity WorldExits for entities the player is close enough to.
    pub fn handle_proximity_exits(game_instance: &GameInstance) {
        let game = &game_instance.game;
        let Some(player_transform) = game.ecs.get_player_transform() else {
            return;
        };
        let player_pos = player_transform.position;

        let exits: Vec<(Entity, WorldExitTrigger)> = game
            .ecs
            .get_store::<WorldExit>()
            .data
            .iter()
            .filter_map(|(&entity, exit)| {
                if !game.entity_in_active_world(entity) {
                    return None;
                }
                if matches!(exit.trigger, WorldExitTrigger::OnProximity(_)) {
                    Some((entity, exit.trigger.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (entity, trigger) in exits {
            let WorldExitTrigger::OnProximity(range) = trigger else {
                continue;
            };
            let Some(transform) = game.ecs.get::<Transform>(entity) else {
                continue;
            };
            let dx = player_pos.x - transform.position.x;
            let dy = player_pos.y - transform.position.y;
            if dx * dx + dy * dy > range * range {
                continue;
            }
            let Some(exit) = game.ecs.get::<WorldExit>(entity) else {
                continue;
            };
            let Some(dest) = exit.destination_world else {
                continue;
            };
            let subject = match exit.mode {
                WorldTransitionMode::Transport => game.ecs.get_player_entity(),
                WorldTransitionMode::Activate => None,
            };
            set_pending_world_transition(WorldTransitionRequest {
                entity: subject,
                world: WorldSelector::ById(dest),
                entry_name: exit.entry.clone(),
                mode: exit.mode,
            });
            break; // at most one exit fires per frame
        }
    }
}
