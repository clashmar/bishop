use super::*;

fn enter_room_mode(room_id: RoomId) {
    with_editor(|editor| {
        let world_id = editor
            .game
            .current_world_id
            .expect("test editor should have a current world");
        editor
            .game
            .get_world_mut(world_id)
            .expect("world should exist")
            .current_room_id = Some(room_id);
        editor.mode = EditorMode::Room(room_id);
        editor.cur_world_id = Some(world_id);
        editor.cur_room_id = Some(room_id);
    });
}

fn enable_back_layer(room_id: RoomId) {
    with_editor(|editor| {
        editor
            .game
            .current_world_mut()
            .expect("world should exist")
            .get_room_mut(room_id)
            .expect("room should exist")
            .current_variant_mut()
            .layers
            .back = Some(BackRoomLayer::default());
    });
}

fn spawn_room_entity(room_id: RoomId, layer: RoomLayer, name: &str, position: Vec2) -> Entity {
    with_editor(|editor| {
        editor
            .game
            .ecs
            .create_entity()
            .with(Name(name.to_string()))
            .with(Transform {
                position,
                ..Default::default()
            })
            .with_current_room_layer(room_id, layer)
            .finish()
    })
}

fn add_second_room() -> RoomId {
    with_editor(|editor| {
        let world_id = editor.cur_world_id.expect("room mode should set a world");
        let room_id = editor.game.id_allocator.allocate_room_id();
        let grid_size = editor.game.current_world().grid_size;
        let room = Room::new(&mut editor.game.ecs, room_id, grid_size);
        editor
            .game
            .get_world_mut(world_id)
            .expect("world should exist")
            .add_room(room);
        room_id
    })
}

#[test]
fn paste_entity_cmd_when_pasted_into_active_layer_then_destination_layer_wins() {
    let _ctx = setup_editor("paste_cmd_active_layer");
    let room_id = with_editor(|editor| {
        editor
            .game
            .current_world()
            .rooms()
            .first()
            .map(|room| room.id)
            .expect("test editor should have a room")
    });
    enter_room_mode(room_id);
    enable_back_layer(room_id);

    let source = spawn_room_entity(room_id, RoomLayer::Front, "Crate", Vec2::new(32.0, 48.0));

    with_editor(|editor| {
        assert!(copy_entity(&mut editor.game.ecs, source));
        editor.room_editor.active_layer_state.active_layer = RoomLayer::Back;
    });

    push_command(Box::new(PasteEntityCmd::new(
        EditorMode::Room(room_id),
        room_id,
        RoomLayer::Back,
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let pasted = editor
            .room_editor
            .single_selected_entity()
            .expect("paste should select the pasted root");
        assert_ne!(pasted, source);
        assert_eq!(
            editor.game.ecs.get::<CurrentRoom>(pasted).map(|room| room.layer),
            Some(RoomLayer::Back)
        );
        assert!(editor
            .game
            .ecs
            .entities_in_room_layer(room_id, RoomLayer::Back)
            .contains(&pasted));
        assert!(editor
            .game
            .ecs
            .entities_in_room_layer(room_id, RoomLayer::Front)
            .contains(&source));
    });
}

#[test]
fn paste_entity_cmd_when_pasted_in_another_room_then_destination_room_wins() {
    let _ctx = setup_editor("paste_cmd_current_room");
    let first_room_id = with_editor(|editor| {
        editor
            .game
            .current_world()
            .rooms()
            .first()
            .map(|room| room.id)
            .expect("test editor should have a room")
    });
    enter_room_mode(first_room_id);
    let second_room_id = add_second_room();

    let source = spawn_room_entity(first_room_id, RoomLayer::Front, "Lamp", Vec2::new(64.0, 96.0));
    with_editor(|editor| {
        assert!(copy_entity(&mut editor.game.ecs, source));
    });

    enter_room_mode(second_room_id);
    push_command(Box::new(PasteEntityCmd::new(
        EditorMode::Room(second_room_id),
        second_room_id,
        RoomLayer::Front,
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let pasted = editor
            .room_editor
            .single_selected_entity()
            .expect("paste should select the pasted root");
        assert_ne!(pasted, source);
        assert_eq!(
            editor.game.ecs.get::<CurrentRoom>(pasted).map(|room| room.room_id),
            Some(second_room_id)
        );
        assert!(editor.game.ecs.entities_in_room(second_room_id).contains(&pasted));
        assert!(editor.game.ecs.entities_in_room(first_room_id).contains(&source));
    });
}

#[test]
fn paste_entity_cmd_when_undone_after_destination_aware_paste_then_pasted_entity_is_removed() {
    let _ctx = setup_editor("paste_cmd_undo");
    let room_id = with_editor(|editor| {
        editor
            .game
            .current_world()
            .rooms()
            .first()
            .map(|room| room.id)
            .expect("test editor should have a room")
    });
    enter_room_mode(room_id);
    enable_back_layer(room_id);

    let source = spawn_room_entity(room_id, RoomLayer::Front, "Switch", Vec2::new(16.0, 16.0));
    let unrelated = spawn_room_entity(room_id, RoomLayer::Front, "Crate", Vec2::new(80.0, 16.0));
    with_editor(|editor| {
        assert!(copy_entity(&mut editor.game.ecs, source));
        editor.room_editor.active_layer_state.active_layer = RoomLayer::Back;
    });

    push_command(Box::new(PasteEntityCmd::new(
        EditorMode::Room(room_id),
        room_id,
        RoomLayer::Back,
    )));
    apply_pending_commands();

    let pasted = with_editor(|editor| {
        editor
            .room_editor
            .single_selected_entity()
            .expect("paste should select the pasted root")
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.game.ecs.get::<Name>(pasted).is_none());
        assert!(!editor
            .game
            .ecs
            .entities_in_room_layer(room_id, RoomLayer::Back)
            .contains(&pasted));
        assert!(editor.game.ecs.get::<Name>(unrelated).is_some());
        assert!(editor
            .game
            .ecs
            .entities_in_room_layer(room_id, RoomLayer::Front)
            .contains(&unrelated));
        assert_eq!(editor.room_editor.single_selected_entity(), None);
    });
}
