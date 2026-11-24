use super::super::*;
use engine_core::storage::editor_config::{get_room_preview, EDITOR_CONFIG};
use engine_core::worlds::InteriorZoneId;

fn set_room_preview_visibility_in_memory(visible: bool) {
    let Ok(mut config) = EDITOR_CONFIG.write() else {
        panic!("editor config lock should be available in this test");
    };
    config.room_preview.interior_zones_visible = visible;
}

#[test]
fn room_editor_when_preview_preference_is_hidden_then_starts_hidden() {
    let previous_visibility = get_room_preview().interior_zones_visible;
    set_room_preview_visibility_in_memory(false);
    let editor = RoomEditor::new();
    set_room_preview_visibility_in_memory(previous_visibility);

    assert!(!editor.show_interior_zones);
}

#[test]
fn entering_zone_sub_mode_selects_room_and_preserves_zone_selection() {
    let mut editor = RoomEditor::new();
    editor.set_selected_entity(Some(Entity(7)));
    editor.drag_state.box_select_active = true;
    editor.interior_zone_editor.selected_zone_id = Some(InteriorZoneId(3));

    editor.set_scene_sub_mode(RoomSceneSubMode::Zones);

    assert_eq!(editor.scene_sub_mode, RoomSceneSubMode::Zones);
    assert!(!editor.inspector.has_target());
    assert!(!editor.drag_state.box_select_active);
    assert_eq!(editor.interior_zone_editor.selected_zone_id, Some(InteriorZoneId(3)));
}

#[test]
fn leaving_zone_sub_mode_clears_zone_state_and_restores_entity_target() {
    let mut editor = RoomEditor::new();
    editor.set_selected_entity(Some(Entity(9)));
    editor.interior_zone_editor.selected_zone_id = Some(InteriorZoneId(5));

    editor.set_scene_sub_mode(RoomSceneSubMode::Zones);
    editor.set_scene_sub_mode(RoomSceneSubMode::Scene);

    assert_eq!(editor.scene_sub_mode, RoomSceneSubMode::Scene);
    assert_eq!(editor.interior_zone_editor.selected_zone_id, None);
    assert!(editor.inspector.has_target());
}

#[test]
fn toggling_zone_sub_mode_twice_exits_and_restores_entity_target() {
    let mut editor = RoomEditor::new();
    editor.set_selected_entity(Some(Entity(11)));
    editor.interior_zone_editor.selected_zone_id = Some(InteriorZoneId(6));

    editor.toggle_zone_sub_mode();
    assert_eq!(editor.scene_sub_mode, RoomSceneSubMode::Zones);
    assert!(!editor.inspector.has_target());
    assert_eq!(editor.interior_zone_editor.selected_zone_id, Some(InteriorZoneId(6)));

    editor.toggle_zone_sub_mode();
    assert_eq!(editor.scene_sub_mode, RoomSceneSubMode::Scene);
    assert_eq!(editor.interior_zone_editor.selected_zone_id, None);
    assert!(editor.inspector.has_target());
}

#[test]
fn interior_zone_visibility_toggle_flips_flag() {
    let mut editor = RoomEditor::new();
    editor.show_interior_zones = true;

    editor.toggle_interior_zone_visibility();
    assert!(!editor.show_interior_zones);

    editor.toggle_interior_zone_visibility();
    assert!(editor.show_interior_zones);
}

#[test]
fn interior_zone_visibility_hiding_exits_zone_sub_mode() {
    let mut editor = RoomEditor::new();
    editor.interior_zone_editor.selected_zone_id = Some(InteriorZoneId(12));
    editor.set_scene_sub_mode(RoomSceneSubMode::Zones);

    editor.set_interior_zone_visibility(false);

    assert_eq!(editor.scene_sub_mode, RoomSceneSubMode::Scene);
    assert_eq!(editor.interior_zone_editor.selected_zone_id, None);
    assert!(!editor.show_interior_zones);
}

#[test]
fn entering_zone_sub_mode_forces_interior_zone_visibility_on() {
    let mut editor = RoomEditor::new();
    editor.show_interior_zones = false;

    editor.set_scene_sub_mode(RoomSceneSubMode::Zones);

    assert!(editor.show_interior_zones);
}
