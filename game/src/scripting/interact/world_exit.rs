use crate::engine::game_instance::GameInstance;
use crate::game_global::set_pending_world_transition;
use crate::scripting::interact::InteractEntry;
use crate::transitions::world_transitions::{WorldSelector, WorldTransitionRequest};
use engine_core::ecs::entity::Entity;
use engine_core::ecs::WorldExit;
use engine_core::worlds::ExitDestination;
use engine_core::worlds::world::{WorldExitTrigger, WorldTransitionMode};

fn on_interact(entity: Entity, game_instance: &GameInstance) {
    let ecs = &game_instance.game.ecs;
    let Some(exit) = ecs.get::<WorldExit>(entity) else { return };
    if !matches!(exit.trigger, WorldExitTrigger::OnInteract) { return };
    let dest = match exit.destination {
        Some(ExitDestination::World(id)) => id,
        Some(ExitDestination::Return) => {
            set_pending_world_transition(WorldTransitionRequest {
                entity: None,
                world: WorldSelector::Return,
                entry_name: None,
                mode: WorldTransitionMode::Overlay,
            });
            return;
        }
        _ => return,
    };

    let is_overlay = game_instance.game.get_world(dest)
        .is_some_and(|w| w.overlay);
    let mode = if is_overlay { WorldTransitionMode::Overlay } else { WorldTransitionMode::Transport };

    set_pending_world_transition(WorldTransitionRequest {
        entity: if is_overlay { None } else { ecs.get_player_entity() },
        world: WorldSelector::ById(dest),
        entry_name: exit.entry.clone(),
        mode,
    });
}

inventory::submit! {
    InteractEntry {
        type_name: WorldExit::TYPE_NAME,
        on_interact,
    }
}
