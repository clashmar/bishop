use super::*;

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
fn alt_drag_copy_cmd_when_undone_then_created_entity_is_removed() {
    let _ctx = setup_editor("alt_drag_copy_cmd_undo");
    enter_room_mode();

    let (room_id, created) = with_editor(|editor| {
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        let created = editor
            .game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(32.0, 48.0),
                ..Default::default()
            })
            .with_current_room_layer(room_id, RoomLayer::Front)
            .finish();
        (room_id, created)
    });

    push_command(Box::new(AltDragCopyCmd::new(
        vec![created],
        EditorMode::Room(room_id),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.game.ecs.get::<Transform>(created).is_some());
        assert_eq!(editor.room_editor.single_selected_entity(), Some(created));
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.game.ecs.get::<Transform>(created).is_none());
        assert_eq!(editor.room_editor.single_selected_entity(), None);
    });
}

#[test]
fn alt_drag_copy_cmd_when_redone_then_created_entity_is_recreated() {
    let _ctx = setup_editor("alt_drag_copy_cmd_redo");
    enter_room_mode();

    let (room_id, created_position, created) = with_editor(|editor| {
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        let created_position = Vec2::new(40.0, 56.0);
        let created = editor
            .game
            .ecs
            .create_entity()
            .with(Transform {
                position: created_position,
                ..Default::default()
            })
            .with_current_room_layer(room_id, RoomLayer::Front)
            .finish();
        (room_id, created_position, created)
    });

    push_command(Box::new(AltDragCopyCmd::new(
        vec![created],
        EditorMode::Room(room_id),
    )));
    apply_pending_commands();

    request_undo();
    apply_pending_commands();
    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        let recreated = editor
            .room_editor
            .single_selected_entity()
            .expect("redo should select the recreated entity");
        let position = editor
            .game
            .ecs
            .get::<Transform>(recreated)
            .map(|transform| transform.position)
            .expect("redo should recreate the transform");
        assert_eq!(position, created_position);
    });
}
