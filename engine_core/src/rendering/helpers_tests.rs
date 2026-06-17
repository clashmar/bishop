use super::*;
use crate::rendering::test_support::make_spillover_entity;
use crate::worlds::test_utils::make_room;
use crate::worlds::{Exit, WorldId};

#[test]
fn visual_position_returns_transform_position_without_subpixel() {
    let position = Vec2::new(10.0, 12.0);

    assert_eq!(visual_position(position, None), position);
}

#[test]
fn visual_position_adds_positive_subpixel_remainder() {
    let position = Vec2::new(10.0, 12.0);
    let sub_pixel = SubPixel { x: 0.25, y: 0.5 };

    assert_eq!(
        visual_position(position, Some(&sub_pixel)),
        Vec2::new(10.25, 12.5)
    );
}

#[test]
fn visual_position_adds_negative_subpixel_remainder() {
    let position = Vec2::new(10.0, 12.0);
    let sub_pixel = SubPixel { x: -0.5, y: -0.25 };

    assert_eq!(
        visual_position(position, Some(&sub_pixel)),
        Vec2::new(9.5, 11.75)
    );
}

#[test]
fn resolve_visual_entity_returns_player_for_proxy() {
    let mut ecs = Ecs::default();
    let player = ecs.create_entity().with(Player).finish();
    let proxy = ecs.create_entity().with(PlayerProxy).finish();

    assert_eq!(resolve_visual_entity(&ecs, proxy), player);
}

#[test]
fn entity_dimensions_use_player_visuals_for_proxy() {
    let mut ecs = Ecs::default();
    let player = ecs
        .create_entity()
        .with(Player)
        .with(CurrentFrame {
            frame_size: vec2(6.0, 16.0),
            ..Default::default()
        })
        .finish();
    let proxy = ecs.create_entity().with(PlayerProxy).finish();
    let sprite_manager = SpriteManager::default();

    assert_eq!(resolve_visual_entity(&ecs, proxy), player);
    assert_eq!(
        entity_dimensions(&ecs, &sprite_manager, proxy, 8.0),
        vec2(6.0, 16.0),
    );
}

#[test]
fn cross_room_visibility_candidate_room_ids_include_current_and_neighbors() {
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![
            make_room(Some(1), 0.0, 0.0, 3.0, 3.0),
            make_room(Some(2), 3.0, 0.0, 3.0, 3.0),
        ],
        1.0,
    );
    let room = world.get_room(RoomId(1)).unwrap();

    assert_eq!(spillover_candidate_room_ids(&world, room), vec![RoomId(1), RoomId(2)]);
}

#[test]
fn cross_room_visibility_entity_visual_overlaps_room_until_fully_outside() {
    let mut ecs = Ecs::default();
    let entity = make_spillover_entity(&mut ecs, Vec2::ZERO, None);
    let room = make_room(Some(1), 0.0, 0.0, 4.0, 4.0);
    let sprite_manager = SpriteManager::default();

    assert!(entity_visual_overlaps_room(
        &ecs,
        &sprite_manager,
        entity,
        vec2(32.0, 63.0),
        &room,
        16.0,
    ));
    assert!(!entity_visual_overlaps_room(
        &ecs,
        &sprite_manager,
        entity,
        vec2(32.0, 96.0),
        &room,
        16.0,
    ));
}

#[test]
fn cross_room_visibility_requires_exit_cell_for_other_room_entity() {
    let room_a = make_room(Some(1), 0.0, 0.0, 4.0, 4.0);
    let room_b = make_room(Some(2), 0.0, 64.0, 4.0, 4.0);
    let world = World::from_rooms(WorldId(0), String::new(), vec![room_a.clone(), room_b], 16.0);
    let mut ecs = Ecs::default();
    let entity = make_spillover_entity(&mut ecs, Vec2::ZERO, None);
    let sprite_manager = SpriteManager::default();

    assert!(!entity_visible_in_room(
        &ecs,
        &sprite_manager,
        &world,
        entity,
        RoomId(2),
        vec2(32.0, 64.0),
        &room_a,
        16.0,
    ));
}

#[test]
fn cross_room_visibility_allows_vertical_exit_cell_spillover() {
    let mut room_a = make_room(Some(1), 0.0, 0.0, 4.0, 4.0);
    room_a.exits.push(Exit {
        position: vec2(1.0, 4.0),
        direction: ExitDirection::Down,
        target_room_id: Some(RoomId(2)),
    });
    let room_b = make_room(Some(2), 0.0, 64.0, 4.0, 4.0);
    let world = World::from_rooms(WorldId(0), String::new(), vec![room_a.clone(), room_b], 16.0);
    let mut ecs = Ecs::default();
    let entity = make_spillover_entity(&mut ecs, Vec2::ZERO, None);
    let sprite_manager = SpriteManager::default();

    assert!(entity_visible_in_room(
        &ecs,
        &sprite_manager,
        &world,
        entity,
        RoomId(2),
        vec2(32.0, 64.0),
        &room_a,
        16.0,
    ));
}

#[test]
fn cross_room_visibility_rejects_overlap_away_from_exit_cell() {
    let mut room_a = make_room(Some(1), 0.0, 0.0, 4.0, 4.0);
    room_a.exits.push(Exit {
        position: vec2(3.0, 4.0),
        direction: ExitDirection::Down,
        target_room_id: Some(RoomId(2)),
    });
    let room_b = make_room(Some(2), 0.0, 64.0, 4.0, 4.0);
    let world = World::from_rooms(WorldId(0), String::new(), vec![room_a.clone(), room_b], 16.0);
    let mut ecs = Ecs::default();
    let entity = make_spillover_entity(&mut ecs, Vec2::ZERO, None);
    let sprite_manager = SpriteManager::default();

    assert!(!entity_visible_in_room(
        &ecs,
        &sprite_manager,
        &world,
        entity,
        RoomId(2),
        vec2(8.0, 64.0),
        &room_a,
        16.0,
    ));
}

#[test]
fn cross_room_visibility_allows_horizontal_exit_cell_spillover() {
    let mut room_a = make_room(Some(1), 0.0, 0.0, 4.0, 4.0);
    room_a.exits.push(Exit {
        position: vec2(4.0, 1.0),
        direction: ExitDirection::Right,
        target_room_id: Some(RoomId(2)),
    });
    let room_b = make_room(Some(2), 64.0, 0.0, 4.0, 4.0);
    let world = World::from_rooms(WorldId(0), String::new(), vec![room_a.clone(), room_b], 16.0);
    let mut ecs = Ecs::default();
    let entity = make_spillover_entity(&mut ecs, Vec2::ZERO, None);
    let sprite_manager = SpriteManager::default();

    assert!(entity_visible_in_room(
        &ecs,
        &sprite_manager,
        &world,
        entity,
        RoomId(2),
        vec2(64.0, 32.0),
        &room_a,
        16.0,
    ));
}

#[test]
fn cross_room_visibility_same_room_entity_does_not_require_exit_cell() {
    let room = make_room(Some(1), 0.0, 0.0, 4.0, 4.0);
    let world = World::from_rooms(WorldId(0), String::new(), vec![room.clone()], 16.0);
    let mut ecs = Ecs::default();
    let entity = make_spillover_entity(&mut ecs, Vec2::ZERO, None);
    let sprite_manager = SpriteManager::default();

    assert!(entity_visible_in_room(
        &ecs,
        &sprite_manager,
        &world,
        entity,
        RoomId(1),
        vec2(32.0, 32.0),
        &room,
        16.0,
    ));
}
