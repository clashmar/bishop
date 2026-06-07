use crate::app::EditorMode;
use crate::gui::inspector::shell::Inspector;
use crate::gui::text_input::committed_name_change;
use crate::shared::scene_ui::inspector::{
    is_scene_component_hidden_in_prefab, linked_prefab_instance_state_for_scene_inspector,
    InspectorContext,
};
use engine_core::ecs::*;
use engine_core::ui::*;
use engine_core::worlds::*;
use widgets::InputCommit;

fn create_prefab(prefab_id: PrefabId, name: String) -> PrefabAsset {
    engine_core::prefab::create_prefab(prefab_id, name)
}

#[test]
fn linked_prefab_metadata_is_hidden_in_prefab_mode() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Entity".to_string()))
        .finish();
    ecs.add_component_to_entity(
        entity,
        PrefabInstanceRoot {
            prefab_id: PrefabId(9),
        },
    );

    let mut prefab_manager = PrefabManager::default();
    prefab_manager
        .prefabs
        .insert(PrefabId(9), create_prefab(PrefabId(9), "Crate".to_string()));

    assert!(linked_prefab_instance_state_for_scene_inspector(
        true,
        &mut ecs,
        &prefab_manager,
        entity,
    )
    .is_some());
    assert!(linked_prefab_instance_state_for_scene_inspector(
        false,
        &mut ecs,
        &prefab_manager,
        entity,
    )
    .is_none());
}

#[test]
fn prefab_selected_entity_create_request_uses_selected_parent() {
    let _root = Entity(10);
    let selected = Entity(22);
    let ctx = InspectorContext {
        command_mode: EditorMode::Prefab(PrefabId(9)),
        show_linked_prefab_metadata: false,
        hide_room_only_components: true,
        selected_create_parent: Some(selected),
        game_name: None,
        event_tags: Vec::new(),
    };

    assert_eq!(ctx.selected_create_parent, Some(selected));
}

#[test]
fn room_context_is_constructed() {
    let ctx = InspectorContext {
        command_mode: EditorMode::Room(RoomId(1)),
        show_linked_prefab_metadata: true,
        hide_room_only_components: false,
        selected_create_parent: None,
        game_name: None,
        event_tags: Vec::new(),
    };

    assert_eq!(ctx.selected_create_parent, None);
}

#[test]
fn prefab_blocked_component_types_exclude_room_specific_types() {
    let current_room = comp_type_name::<CurrentRoom>();
    let room_camera = comp_type_name::<RoomCamera>();
    let player_proxy = comp_type_name::<PlayerProxy>();

    assert!(is_scene_component_hidden_in_prefab(current_room));
    assert!(is_scene_component_hidden_in_prefab(room_camera));
    assert!(is_scene_component_hidden_in_prefab(player_proxy));

    let player = comp_type_name::<Player>();
    let global = comp_type_name::<Global>();
    assert!(is_scene_component_hidden_in_prefab(player));
    assert!(is_scene_component_hidden_in_prefab(global));
}

#[test]
fn committed_name_change_only_emits_on_committed_edits() {
    assert_eq!(committed_name_change("Old", "New", InputCommit::Committed), Some("New".to_string()));
    assert_eq!(committed_name_change("Old", "Old", InputCommit::Committed), None);
    assert_eq!(committed_name_change("Old", "New", InputCommit::Previewing), None);
}

#[test]
fn room_properties_does_not_create_entity_target() {
    let mut inspector = Inspector::new();
    inspector.select_room();
    assert!(!inspector.has_target());
}
