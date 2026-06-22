use crate::engine::game_instance::GameInstance;
use engine_core::ecs::{Active, Entity};
use engine_core::game::Game;
use engine_core::hydration::{
    self, DerivedTraversalClaims, HydrationCoordinator, HydrationScope
};
use engine_core::worlds::RoomId;
use engine_core::worlds::topology::extract_topology;

/// Sets Active.value per entity based on current room, respecting pin_count.
pub(crate) fn apply_current_room_default_activation(
    game: &mut Game,
) {
    let current_room = game.current_world().current_room_id.unwrap_or_default();

    let mut room_entities: Vec<(RoomId, Vec<Entity>)> = Vec::new();
    for world in game.worlds() {
        for room in world.rooms() {
            let entities: Vec<_> = game.ecs.entities_in_room(room.id).iter().copied().collect();
            room_entities.push((room.id, entities));
        }
    }

    for (room_id, entities) in room_entities {
        for entity in entities {
            if let Some(active) = game.ecs.get_mut::<Active>(entity) {
                if active.pin_count == 0 {
                    active.value = room_id == current_room;
                }
            }
        }
    }
}

/// Replaces traversal-scope coordinator claims from derived traversal claims.
pub(crate) fn sync_coordinator_claims(
    coordinator: &mut HydrationCoordinator,
    claims: &DerivedTraversalClaims,
) {
    for scope in coordinator.active_scopes() {
        if matches!(
            scope,
            HydrationScope::Room(_) | HydrationScope::World(_) | HydrationScope::Entity(_)
        ) {
            coordinator.deactivate_scope(scope);
        }
    }

    for (&room_id, assets) in &claims.room_claims {
        let scope = HydrationScope::Room(room_id);
        coordinator.activate_scope(scope.clone());
        for asset in assets {
            coordinator.claim_asset(scope.clone(), *asset);
        }
    }

    for (&world_id, _assets) in &claims.world_claims {
        let scope = HydrationScope::World(world_id);
        coordinator.activate_scope(scope.clone());
    }

    for (&entity, assets) in &claims.pinned_entity_claims {
        let scope = HydrationScope::Entity(entity);
        coordinator.activate_scope(scope.clone());
        for asset in assets {
            coordinator.claim_asset(scope.clone(), *asset);
        }
    }
}

/// Recomputes traversal residency after a room or world change.
pub fn refresh_after_traversal(
    game_instance: &mut GameInstance,
) {
    let topology = extract_topology(&game_instance.game);
    let claims = hydration::derive_traversal_claims(
        &game_instance.game,
        &topology,
    );

    apply_current_room_default_activation(&mut game_instance.game);
    sync_coordinator_claims(&mut game_instance.game.hydration_coordinator, &claims);
}
