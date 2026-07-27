use super::*;
use engine_core::ecs::{Name, TilePlacement, Transform};
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
fn set_back_layer_enabled_cmd_is_undoable_and_destructive() {
    let _ctx = setup_editor("back_layer_enable_cmd");
    enter_room_mode();

    let (room_id, back_entity, tile_id) = with_editor(|editor| {
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
            sprite_id: SpriteId(7),
            components: Vec::new(),
        });

        let back_entity = editor
            .game
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Back Entity".to_string()))
            .with_current_room_layer(room_id, RoomLayer::Back)
            .finish();

        editor
            .game
            .ecs
            .create_entity()
            .with(TilePlacement::new(tile_id, 3, 2))
            .with_current_room_layer(room_id, RoomLayer::Back)
            .finish();

        (room_id, back_entity, tile_id)
    });

    push_command(Box::new(SetBackLayerEnabledCmd::new(room_id, false)));
    apply_pending_commands();

    with_editor(|editor| {
        let room = editor
            .game
            .current_world()
            .get_room(room_id)
            .expect("room should exist");
        assert!(room.current_variant().layers.back.is_none());
        assert!(editor.game.ecs.get::<Name>(back_entity).is_none());
        assert_eq!(editor.game.ecs.tile_entity_at(room_id, RoomLayer::Back, 3, 2), None);
        assert!(editor.game.tile_registry.get(tile_id).is_some());
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        let room = editor
            .game
            .current_world()
            .get_room(room_id)
            .expect("room should exist");
        assert!(room.current_variant().layers.back.is_some());
        assert!(editor.game.ecs.get::<Name>(back_entity).is_some());
        assert!(editor.game.ecs.tile_entity_at(room_id, RoomLayer::Back, 3, 2).is_some());
    });
}
