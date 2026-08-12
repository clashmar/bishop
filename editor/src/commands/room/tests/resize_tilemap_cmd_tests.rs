use super::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::{BackRoomLayer, InteriorZoneBounds};

#[test]
fn resize_tilemap_cmd_rejects_resize_when_interior_zone_would_leave_room_bounds() {
    let _ctx = setup_editor("resize_tilemap_cmd_zone_bounds");
    enter_room_mode();

    let (room_id, variant_index, old_position, old_size, old_width, old_height) = with_editor(|editor| {
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        let room = editor
            .game
            .current_world_mut()
            .expect("world should exist")
            .get_room_mut(room_id)
            .expect("room should exist");
        room.current_variant_mut().layers.back = Some(BackRoomLayer {
            interior_zones: vec![InteriorZone {
                id: InteriorZoneId(1),
                bounds: InteriorZoneBounds::new(32, 0, 16, 16),
            }],
            ..Default::default()
        });
        room.size = Vec2::new(4.0, 4.0);
        room.current_variant_mut().tilemap = TileMap::new(4, 4);
        (
            room_id,
            room.current_variant_index(),
            room.position,
            room.size,
            room.current_variant().tilemap.width,
            room.current_variant().tilemap.height,
        )
    });

    push_command(Box::new(ResizeTilemapCmd::new(
        room_id,
        variant_index,
        HandleSide::Right,
        -3,
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let room = editor
            .game
            .current_world()
            .get_room(room_id)
            .expect("room should exist");
        assert_eq!(room.position, old_position);
        assert_eq!(room.size, old_size);
        assert_eq!(room.current_variant().tilemap.width, old_width);
        assert_eq!(room.current_variant().tilemap.height, old_height);
    });
}
