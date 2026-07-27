use super::*;
use engine_core::worlds::{BackRoomLayer, RoomLayer};

fn enter_room_mode() {
    with_editor(|editor| {
        let world_id = editor
            .game
            .current_world_id
            .expect("test editor should have a current world");
        let room_id = editor
            .game
            .current_world()
            .rooms()
            .first()
            .map(|room| room.id)
            .expect("test editor should have a room");
        editor
            .game
            .get_world_mut(world_id)
            .expect("test editor should resolve current world")
            .current_room_id = Some(room_id);
        editor.mode = EditorMode::Room(room_id);
        editor.cur_world_id = Some(world_id);
        editor.cur_room_id = Some(room_id);
    });
}

#[test]
fn set_tile_placement_cmd_when_placed_then_runtime_components_apply_and_undo_restores_previous_state() {
    let _ctx = setup_editor("tile_placement_cmd");
    enter_room_mode();

    let (room_id, tile_id) = with_editor(|editor| {
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        let tile_id = editor.game.tile_registry.insert(TileDef {
            sprite_id: SpriteId(5),
            components: vec![tile_definition_component_snapshot(Solid(true))],
        });
        (room_id, tile_id)
    });

    push_command(Box::new(SetTilePlacementCmd::place(
        room_id,
        RoomLayer::Front,
        (2, 3),
        tile_id,
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let entity = editor
            .game
            .ecs
            .tile_entity_at(room_id, RoomLayer::Front, 2, 3)
            .expect("tile placement should exist after execute");
        let placed = editor
            .game
            .ecs
            .tile_placement_at(room_id, RoomLayer::Front, 2, 3)
            .expect("tile placement should resolve through the room and cell index");

        assert_eq!(placed.definition, tile_id);
        assert_eq!((placed.grid_x, placed.grid_y), (2, 3));
        assert!(editor.game.ecs.get::<Solid>(entity).is_some_and(|solid| solid.0));
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(editor.game.ecs.tile_entity_at(room_id, RoomLayer::Front, 2, 3), None);
    });
}

#[test]
fn set_tile_placement_cmd_targets_the_requested_layer() {
    let _ctx = setup_editor("tile_placement_cmd_requested_layer");
    enter_room_mode();

    let (room_id, tile_id) = with_editor(|editor| {
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        editor
            .game
            .current_world_mut()
            .expect("world should exist")
            .get_room_mut(room_id)
            .expect("room should exist")
            .current_variant_mut()
            .layers
            .back = Some(BackRoomLayer::default());
        let tile_id = editor.game.tile_registry.insert(TileDef {
            sprite_id: SpriteId(8),
            components: Vec::new(),
        });
        (room_id, tile_id)
    });

    push_command(Box::new(SetTilePlacementCmd::place(
        room_id,
        RoomLayer::Back,
        (4, 5),
        tile_id,
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor
            .game
            .ecs
            .tile_entity_at(room_id, RoomLayer::Back, 4, 5)
            .is_some());
        assert_eq!(editor.game.ecs.tile_entity_at(room_id, RoomLayer::Front, 4, 5), None);
    });
}

#[test]
fn set_tile_placement_cmd_when_one_linked_tile_is_cleared_then_sibling_placement_stays_intact() {
    let _ctx = setup_editor("tile_placement_sibling_independence");
    enter_room_mode();

    let (room_id, tile_id) = with_editor(|editor| {
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        let tile_id = editor.game.tile_registry.insert(TileDef {
            sprite_id: SpriteId(6),
            components: vec![tile_definition_component_snapshot(Solid(true))],
        });
        (room_id, tile_id)
    });

    push_command(Box::new(SetTilePlacementCmd::place(
        room_id,
        RoomLayer::Front,
        (1, 1),
        tile_id,
    )));
    push_command(Box::new(SetTilePlacementCmd::place(
        room_id,
        RoomLayer::Front,
        (2, 1),
        tile_id,
    )));
    apply_pending_commands();

    push_command(Box::new(SetTilePlacementCmd::clear(
        room_id,
        RoomLayer::Front,
        (1, 1),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(editor.game.ecs.tile_entity_at(room_id, RoomLayer::Front, 1, 1), None);

        let sibling = editor
            .game
            .ecs
            .tile_entity_at(room_id, RoomLayer::Front, 2, 1)
            .expect("sibling placement should remain after clearing one linked tile");
        assert!(editor.game.ecs.get::<Solid>(sibling).is_some_and(|solid| solid.0));
    });
}
