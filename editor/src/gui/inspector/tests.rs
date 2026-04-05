use crate::app::EditorMode;
use crate::shared::scene_ui::inspector::{
    is_scene_component_hidden_in_prefab, linked_prefab_instance_state_for_scene_inspector,
    SceneEmptyInspectorBehavior, SceneInspectorContext, SceneInspectorOutput, ScenePrefabAction,
};
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

    assert!(linked_prefab_instance_state_for_scene_inspector(
        true,
        &mut ecs,
        &prefab_library,
        entity,
    )
    .is_some());
    assert!(linked_prefab_instance_state_for_scene_inspector(
        false,
        &mut ecs,
        &prefab_library,
        entity,
    )
    .is_none());
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

#[test]
fn linked_prefab_state_for_scene_inspector_resolves_child_selection_to_root() {
    let mut ecs = Ecs::default();
    let root = ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Root".to_string()))
        .finish();
    let child = ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Child".to_string()))
        .finish();
    set_parent(&mut ecs, child, root);

    let prefab_id = PrefabId(12);
    ecs.add_component_to_entity(root, PrefabInstanceRoot { prefab_id });
    ecs.add_component_to_entity(
        root,
        PrefabInstanceNode {
            prefab_id,
            node_id: 1,
            root_entity: root,
        },
    );
    ecs.add_component_to_entity(
        child,
        PrefabInstanceNode {
            prefab_id,
            node_id: 2,
            root_entity: root,
        },
    );
    ecs.add_component_to_entity(
        child,
        PrefabOverrides {
            modified_components: vec![Transform::TYPE_NAME.to_string()],
            ..Default::default()
        },
    );

    let mut prefab_library = PrefabLibrary::default();
    prefab_library
        .prefabs
        .insert(prefab_id, create_prefab(prefab_id, "Crate".to_string()));

    let state = linked_prefab_instance_state_for_scene_inspector(
        true,
        &mut ecs,
        &prefab_library,
        child,
    )
    .expect("linked child should expose room inspector prefab state");

    assert_eq!(state.root_entity, root);
    assert_eq!(state.prefab_id, prefab_id);
    assert_eq!(state.prefab_name, "Crate");
    assert!(state.has_overrides);
    assert_eq!(state.open_action, ScenePrefabAction::OpenPrefabEditor);
}

#[test]
fn scene_inspector_output_defaults_to_no_prefab_picker_request() {
    let output = SceneInspectorOutput::default();

    assert!(!output.open_prefab_picker);
}
