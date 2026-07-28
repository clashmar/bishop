use super::*;
use crate::rendering::test_support::make_spillover_entity;
use crate::worlds::test_utils::make_room;
use crate::worlds::{Exit, WorldId};

// --- smooth_dt tests ---

#[test]
fn smooth_dt_skips_first_frame_and_seeds_from_second() {
    let mut state = SmoothedDtState::default();
    // First frame: init gap of 500ms — should be discarded for EMA purposes.
    let first = smooth_dt(&mut state, 0.5, 0.9);
    // Returned value is snap_dt(0.5) = 0.5 (no frequency match),
    // but the accumulator caps it via MAX_ACCUM externally.
    assert_eq!(first, 0.5);
    assert!(matches!(state, SmoothedDtState::AwaitingSeed));

    // Second frame: real inter-frame delta of ~16.67ms (60Hz).
    let second = smooth_dt(&mut state, 1.0 / 60.0, 0.9);
    // snap_dt(16.67ms) snaps to 1/60.
    assert!((second - 1.0 / 60.0).abs() < 1e-6);
    assert!(matches!(state, SmoothedDtState::Active(_)));
}

#[test]
fn smooth_dt_ema_converges_from_good_seed() {
    let mut state = SmoothedDtState::default();
    // Discard first frame.
    smooth_dt(&mut state, 0.5, 0.9);
    // Seed from second frame at 60Hz.
    let dt_60 = 1.0 / 60.0;
    smooth_dt(&mut state, dt_60, 0.9);

    // Subsequent frames: EMA should stay close to 1/60.
    for _ in 0..10 {
        let result = smooth_dt(&mut state, dt_60, 0.9);
        assert!((result - dt_60).abs() < 0.001, "EMA drifted: {result}");
    }
}

#[test]
fn smooth_dt_first_frame_does_not_pollute_ema() {
    let mut state = SmoothedDtState::default();
    // First frame: huge init gap.
    smooth_dt(&mut state, 0.5, 0.9);
    // Second frame: seed from real dt.
    let dt_60 = 1.0 / 60.0;
    let _second = smooth_dt(&mut state, dt_60, 0.9);

    // Third frame: EMA should be near 1/60.
    let third = smooth_dt(&mut state, dt_60, 0.9);
    assert!((third - dt_60).abs() < 0.001, "EMA should be near 1/60, got {third}");
}

#[test]
fn smooth_dt_handles_120hz_display() {
    let mut state = SmoothedDtState::default();
    // First frame: discard.
    smooth_dt(&mut state, 0.3, 0.9);
    // Second frame: 120Hz delta.
    let dt_120 = 1.0 / 120.0;
    let second = smooth_dt(&mut state, dt_120, 0.9);
    assert!((second - dt_120).abs() < 1e-6);

    // Third frame: EMA should be near 120Hz.
    let third = smooth_dt(&mut state, dt_120, 0.9);
    assert!((third - dt_120).abs() < 0.001, "EMA should be near 1/120, got {third}");
}

#[test]
fn smooth_dt_state_transitions_awaiting_first_to_awaiting_seed_to_active() {
    let mut state = SmoothedDtState::AwaitingFirstFrame;
    assert!(matches!(state, SmoothedDtState::AwaitingFirstFrame));

    smooth_dt(&mut state, 0.016, 0.9);
    assert!(matches!(state, SmoothedDtState::AwaitingSeed));

    smooth_dt(&mut state, 0.016, 0.9);
    assert!(matches!(state, SmoothedDtState::Active(_)));
}

// --- snap_dt tests ---

#[test]
fn snap_dt_snaps_to_60hz() {
    let dt = 1.0 / 60.0;
    assert!((snap_dt(dt) - dt).abs() < 1e-10);
}

#[test]
fn snap_dt_snaps_near_60hz() {
    let slightly_off = 1.0 / 60.0 + 0.001;
    assert!((snap_dt(slightly_off) - 1.0 / 60.0).abs() < 1e-10);
}

#[test]
fn snap_dt_does_not_snap_far_values() {
    let large = 0.5;
    assert!((snap_dt(large) - 0.5).abs() < 1e-10);
}

#[test]
fn snap_dt_snaps_to_120hz() {
    let dt = 1.0 / 120.0;
    assert!((snap_dt(dt) - dt).abs() < 1e-10);
}

#[test]
fn snap_dt_snaps_to_144hz() {
    let dt = 1.0 / 144.0;
    assert!((snap_dt(dt) - dt).abs() < 1e-10);
}

#[test]
fn snap_dt_snaps_to_30hz() {
    let dt = 1.0 / 30.0;
    assert!((snap_dt(dt) - dt).abs() < 1e-10);
}

// --- original tests below ---

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
fn entity_visual_rect_includes_current_frame_offset() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(Transform {
            position: vec2(16.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(CurrentFrame {
            frame_size: vec2(8.0, 8.0),
            offset: vec2(2.0, 3.0),
            ..Default::default()
        })
        .finish();

    assert_eq!(
        entity_visual_rect(&ecs, &SpriteManager::default(), entity, vec2(16.0, 16.0), 16.0),
        Rect::new(18.0, 19.0, 8.0, 8.0),
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
