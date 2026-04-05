use crate::app::EditorMode;
use crate::gui::inspector::helpers::{
    is_scene_component_hidden_in_prefab, linked_prefab_metadata_for_scene_inspector,
};
use crate::shared::scene_ui::inspector::{SceneEmptyInspectorBehavior, SceneInspectorContext};
use engine_core::prelude::*;

fn create_prefab(prefab_id: PrefabId, name: String) -> PrefabAsset {
    engine_core::prelude::create_prefab(prefab_id, name)
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

    let mut prefab_library = PrefabLibrary::default();
    prefab_library
        .prefabs
        .insert(PrefabId(9), create_prefab(PrefabId(9), "Crate".to_string()));

    assert!(linked_prefab_metadata_for_scene_inspector(true, &ecs, &prefab_library, entity).is_some());
    assert!(linked_prefab_metadata_for_scene_inspector(false, &ecs, &prefab_library, entity).is_none());
}

#[test]
fn prefab_selected_entity_create_request_uses_selected_parent() {
    let root = Entity(10);
    let selected = Entity(22);
    let ctx = SceneInspectorContext {
        command_mode: EditorMode::Prefab(PrefabId(9)),
        show_linked_prefab_metadata: false,
        hide_room_only_components: true,
        selected_create_parent: Some(selected),
        empty_state: SceneEmptyInspectorBehavior::Prefab {
            fallback_parent: Some(root),
        },
    };

    assert_eq!(ctx.selected_create_parent, Some(selected));
    match ctx.empty_state {
        SceneEmptyInspectorBehavior::Prefab { fallback_parent } => {
            assert_eq!(fallback_parent, Some(root));
        }
        SceneEmptyInspectorBehavior::Room => panic!("expected prefab empty state"),
    }
}

#[test]
fn room_context_keeps_room_empty_state() {
    let ctx = SceneInspectorContext {
        command_mode: EditorMode::Room(RoomId(1)),
        show_linked_prefab_metadata: true,
        hide_room_only_components: false,
        selected_create_parent: None,
        empty_state: SceneEmptyInspectorBehavior::Room,
    };

    assert_eq!(ctx.selected_create_parent, None);
    assert_eq!(ctx.empty_state, SceneEmptyInspectorBehavior::Room);
}

#[test]
fn prefab_blocked_component_types_exclude_room_specific_types() {
    let current_room = comp_type_name::<CurrentRoom>();
    let room_camera = comp_type_name::<RoomCamera>();
    let player_proxy = comp_type_name::<PlayerProxy>();
    let player = comp_type_name::<Player>();
    let global = comp_type_name::<Global>();
    let transform = comp_type_name::<Transform>();

    assert!(is_scene_component_hidden_in_prefab(current_room));
    assert!(is_scene_component_hidden_in_prefab(room_camera));
    assert!(is_scene_component_hidden_in_prefab(player_proxy));
    assert!(is_scene_component_hidden_in_prefab(player));
    assert!(is_scene_component_hidden_in_prefab(global));
    assert!(!is_scene_component_hidden_in_prefab(transform));
}
