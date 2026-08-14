use super::support::{
    setup_physics_down_transition_game, setup_physics_transition_game, step_physics_and_transitions,
};
use engine_core::ecs::CurrentRoom;
use engine_core::worlds::{RoomId, RoomLayer};

#[test]
fn physics_driven_down_transition_changes_room_when_falling_through_exit() {
    let (lua, mut game_instance, entity) = setup_physics_down_transition_game();
    let after = step_physics_and_transitions(&lua, &mut game_instance, entity);
    assert!(after.y > 72.0);
    assert_eq!(
        game_instance
            .game
            .ecs
            .get::<CurrentRoom>(entity)
            .map(|room| room.room_id),
        Some(RoomId(2))
    );
}

#[test]
fn physics_driven_horizontal_transitions_wait_for_visual_boundary_crossing() {
    let (lua, mut right_game, right_entity) =
        setup_physics_transition_game(RoomId(1), 30.0, 100.0);
    let right_after_one = step_physics_and_transitions(&lua, &mut right_game, right_entity);
    assert!(right_after_one.x < 32.0);
    assert_eq!(
        right_game
            .game
            .ecs
            .get::<CurrentRoom>(right_entity)
            .map(|room| room.room_id),
        Some(RoomId(1))
    );
    let right_after_two = step_physics_and_transitions(&lua, &mut right_game, right_entity);
    assert!(right_after_two.x > 32.0);
    assert_eq!(
        right_game
            .game
            .ecs
            .get::<CurrentRoom>(right_entity)
            .map(|room| room.room_id),
        Some(RoomId(2))
    );

    let (lua, mut left_game, left_entity) =
        setup_physics_transition_game(RoomId(2), 34.0, -100.0);
    let left_after_one = step_physics_and_transitions(&lua, &mut left_game, left_entity);
    assert!(left_after_one.x > 32.0);
    assert_eq!(
        left_game
            .game
            .ecs
            .get::<CurrentRoom>(left_entity)
            .map(|room| room.room_id),
        Some(RoomId(2))
    );
    let left_after_two = step_physics_and_transitions(&lua, &mut left_game, left_entity);
    assert!(left_after_two.x < 32.0);
    assert_eq!(
        left_game
            .game
            .ecs
            .get::<CurrentRoom>(left_entity)
            .map(|room| room.room_id),
        Some(RoomId(1))
    );
}

#[test]
fn physics_driven_horizontal_transitions_require_matching_exit_layer() {
    let (lua, mut game_instance, entity) = setup_physics_transition_game(RoomId(1), 22.0, 100.0);
    game_instance
        .game
        .ecs
        .set_current_room_layer(entity, RoomId(1), RoomLayer::Back);

    let after_one = step_physics_and_transitions(&lua, &mut game_instance, entity);
    assert!(after_one.x < 24.0);
    assert_eq!(
        game_instance
            .game
            .ecs
            .get::<CurrentRoom>(entity)
            .map(|room| (room.room_id, room.layer)),
        Some((RoomId(1), RoomLayer::Back))
    );

    let after_two = step_physics_and_transitions(&lua, &mut game_instance, entity);
    assert!(after_two.x <= 24.0);
    assert_eq!(
        game_instance
            .game
            .ecs
            .get::<CurrentRoom>(entity)
            .map(|room| (room.room_id, room.layer)),
        Some((RoomId(1), RoomLayer::Back))
    );
}

#[test]
fn physics_driven_horizontal_transitions_allow_matching_back_exit_layer() {
    let (lua, mut game_instance, entity) = setup_physics_transition_game(RoomId(1), 30.0, 100.0);
    game_instance
        .game
        .ecs
        .set_current_room_layer(entity, RoomId(1), RoomLayer::Back);
    let world = game_instance
        .game
        .current_world_mut()
        .expect("transition test world should exist");
    for room in world.rooms_mut() {
        for exit in &mut room.exits {
            exit.layer = RoomLayer::Back;
        }
    }

    let _after_one = step_physics_and_transitions(&lua, &mut game_instance, entity);
    let after_two = step_physics_and_transitions(&lua, &mut game_instance, entity);
    assert!(after_two.x > 32.0);
    assert_eq!(
        game_instance
            .game
            .ecs
            .get::<CurrentRoom>(entity)
            .map(|room| (room.room_id, room.layer)),
        Some((RoomId(2), RoomLayer::Back))
    );
}
