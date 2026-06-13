use crate::engine::game_instance::GameInstance;
use crate::game_global::set_pending_world_transition;
use crate::scripting::interact::InteractEntry;
use crate::transitions::world_transitions::{WorldSelector, WorldTransitionRequest};
use engine_core::ecs::entity::Entity;
use engine_core::ecs::WorldExit;
use engine_core::worlds::world::{WorldExitTrigger, WorldTransitionMode};

fn on_interact(entity: Entity, game_instance: &GameInstance) {
    let ecs = &game_instance.game.ecs;
    let Some(exit) = ecs.get::<WorldExit>(entity) else { return };
    if !matches!(exit.trigger, WorldExitTrigger::OnInteract) { return };
    let Some(dest) = exit.destination_world else { return };

    let subject = match exit.mode {
        WorldTransitionMode::Transport => ecs.get_player_entity(),
        WorldTransitionMode::Activate => None,
    };

    set_pending_world_transition(WorldTransitionRequest {
        entity: subject,
        world: WorldSelector::ById(dest),
        entry_name: exit.entry.clone(),
        mode: exit.mode,
    });
}

inventory::submit! {
    InteractEntry {
        type_name: WorldExit::TYPE_NAME,
        on_interact,
    }
}
