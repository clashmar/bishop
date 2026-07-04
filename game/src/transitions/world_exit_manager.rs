use crate::engine::game_instance::GameInstance;
use crate::game_global::set_pending_world_transition;
use crate::transitions::world_transitions::{TraversalRequest, WorldSelector};
use engine_core::ecs::*;
use engine_core::worlds::ExitDestination;
use engine_core::worlds::world::{WorldExitTrigger, WorldTransitionMode};

/// Per-frame system that fires pending world transitions triggered by `WorldExit` components.
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
            let dest = match exit.destination {
                Some(ExitDestination::World(id)) => id,
                Some(ExitDestination::Return) => {
                    set_pending_world_transition(TraversalRequest::return_from_world());
                    break;
                }
                _ => continue,
            };
            let is_overlay = game.get_world(dest).is_some_and(|w| w.overlay);
            let mode = if is_overlay { WorldTransitionMode::Overlay } else { WorldTransitionMode::Transport };
            set_pending_world_transition(TraversalRequest::to_world(
                if is_overlay {
                    None
                } else {
                    game.ecs.get_player_entity()
                },
                WorldSelector::ById(dest),
                exit.entry.clone(),
                mode,
            ));
            break;
        }
    }
}
