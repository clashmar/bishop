use super::super::*;
use engine_core::worlds::InteriorZoneId;

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
