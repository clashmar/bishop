use crate::engine::game_instance::GameInstance;
use crate::transitions::traversal_residency;
use engine_core::ecs::{Active, Script, ScriptId};
use engine_core::hydration::{HydrationScope, ResourceClass};
use engine_core::worlds::{Room, RoomId, World, WorldId};
use std::collections::HashMap;

fn room_bound_entity(
    game: &mut engine_core::game::Game,
    room_id: RoomId,
    pinned: bool,
) -> engine_core::ecs::Entity {
    let entity = game
        .ecs
        .create_entity()
        .with(Active::new(false))
        .with(Script {
            script_id: ScriptId(1),
            ..Default::default()
        })
        .with_current_room(room_id)
        .finish();
    if pinned {
        game.ecs.get_mut::<Active>(entity).unwrap().pin();
    }
    entity
}

fn two_room_game() -> engine_core::game::Game {
    let mut world = World::default();
    world.id = WorldId(1);
    world.current_room_id = Some(RoomId(1));
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    world.add_room(Room {
        id: RoomId(2),
        ..Default::default()
    });

    let mut game = engine_core::game::Game::default();
    game.add_world(world);
    game
}

#[test]
fn room_transition_activates_current_room_and_deactivates_unpinned_previous_room_entities() {
    let mut game = two_room_game();
    let old_room_entity = room_bound_entity(&mut game, RoomId(1), false);
    let new_room_entity = room_bound_entity(&mut game, RoomId(2), false);

    game.current_world_mut().unwrap().current_room_id = Some(RoomId(2));

    let mut instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };
    traversal_residency::refresh_after_traversal(&mut instance);

    assert!(instance.game.ecs.get::<Active>(new_room_entity).unwrap().value);
    assert!(!instance.game.ecs.get::<Active>(old_room_entity).unwrap().value);
}

#[test]
fn pinned_entity_is_not_deactivated_or_unclaimed_when_player_leaves_its_room() {
    let mut game = two_room_game();
    let hunter = room_bound_entity(&mut game, RoomId(2), true);

    game.current_world_mut().unwrap().current_room_id = Some(RoomId(1));

    let mut instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };
    traversal_residency::refresh_after_traversal(&mut instance);

    assert!(instance.game.ecs.get::<Active>(hunter).unwrap().is_enabled());
    assert!(
        instance
            .game
            .hydration_coordinator
            .claim_count(&HydrationScope::Entity(hunter), ResourceClass::Script)
            > 0
    );
}

#[test]
fn pinned_room_payload_stays_claimed_outside_the_frontier() {
    let mut game = two_room_game();
    let hunter = room_bound_entity(&mut game, RoomId(2), true);
    game.current_world_mut().unwrap().current_room_id = Some(RoomId(1));

    let mut instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };
    traversal_residency::refresh_after_traversal(&mut instance);

    assert!(
        instance
            .game
            .hydration_coordinator
            .claim_count(&HydrationScope::Entity(hunter), ResourceClass::RoomPayload)
            > 0
    );
}

#[test]
fn unpin_releases_room_payload_on_the_next_refresh_when_no_other_claim_remains() {
    let mut game = two_room_game();
    let hunter = room_bound_entity(&mut game, RoomId(2), true);
    game.current_world_mut().unwrap().current_room_id = Some(RoomId(1));

    let mut instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };
    traversal_residency::refresh_after_traversal(&mut instance);
    instance.game.ecs.get_mut::<Active>(hunter).unwrap().unpin();
    traversal_residency::refresh_after_traversal(&mut instance);

    assert_eq!(
        instance
            .game
            .hydration_coordinator
            .claim_count(&HydrationScope::Entity(hunter), ResourceClass::RoomPayload),
        0
    );
}
