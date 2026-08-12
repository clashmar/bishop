pub(super) use crate::app::EditorMode;
pub(super) use crate::commands::room::{
    copy_entity,
    AltDragCopyCmd,
    PasteEntityCmd,
    ResizeTilemapCmd,
    SetBackLayerCompositionModeCmd,
    SetBackLayerEnabledCmd,
    SetBackLayerZoneScopeCmd,
    SetTilePlacementCmd,
    UpdateInteriorZonesCmd,
};
pub(super) use crate::editor_global::{
    apply_pending_commands,
    push_command,
    request_redo,
    request_undo,
    take_pending_toast,
    with_editor,
};
pub(super) use crate::test_utils::setup_editor;
pub(super) use bishop::prelude::Vec2;
pub(super) use crate::tilemap::resize_handle::HandleSide;
pub(super) use engine_core::ecs::{CurrentRoom, Entity, Name, Solid, SpriteId, Transform};
pub(super) use engine_core::tiles::{TileDef, tile_definition_component_snapshot};
pub(super) use engine_core::worlds::{
    BackRoomLayer, InteriorZone, InteriorZoneId, Room, RoomId, RoomLayer,
};

mod alt_drag_copy_cmd_tests;
mod back_layer_cmd_tests;
mod paste_cmd_tests;
mod resize_tilemap_cmd_tests;
mod tile_placement_cmd_tests;

pub(super) fn enter_room_mode() {
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
