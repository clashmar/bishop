use super::*;
use bishop::prelude::Color;

#[test]
fn set_room_ambient_cmd_updates_all_settings_and_supports_undo_redo() {
    let _ctx = setup_editor("room_ambient_cmd");
    enter_room_mode();

    let (world_id, room_id) = with_editor(|editor| {
        let world_id = editor
            .game
            .current_world_id
            .expect("test editor should have a current world");
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        let room = editor
            .game
            .current_world_mut()
            .expect("world should exist")
            .get_room_mut(room_id)
            .expect("room should exist");
        room.darkness = 0.2;
        room.ambient_color = Color::new(0.3, 0.4, 0.5, 1.0);
        room.exposure = 0.8;
        (world_id, room_id)
    });
    let old_settings = RoomAmbientSettings {
        darkness: 0.2,
        color: Color::new(0.3, 0.4, 0.5, 1.0),
        exposure: 0.8,
    };
    let new_settings = RoomAmbientSettings {
        darkness: 0.65,
        color: Color::new(0.7, 0.6, 0.5, 1.0),
        exposure: 1.4,
    };

    push_command(Box::new(SetRoomAmbientCmd::new(
        world_id,
        room_id,
        old_settings,
        new_settings,
    )));
    apply_pending_commands();
    with_editor(|editor| {
        let room = editor
            .game
            .current_world()
            .get_room(room_id)
            .expect("room should exist");
        assert_room_ambient(room, new_settings);
    });

    request_undo();
    apply_pending_commands();
    with_editor(|editor| {
        let room = editor
            .game
            .current_world()
            .get_room(room_id)
            .expect("room should exist");
        assert_room_ambient(room, old_settings);
    });

    request_redo();
    apply_pending_commands();
    with_editor(|editor| {
        let room = editor
            .game
            .current_world()
            .get_room(room_id)
            .expect("room should exist");
        assert_room_ambient(room, new_settings);
    });
}

fn assert_room_ambient(room: &Room, expected: RoomAmbientSettings) {
    assert_eq!(room.darkness, expected.darkness);
    assert_eq!(room.ambient_color, expected.color);
    assert_eq!(room.exposure, expected.exposure);
}
