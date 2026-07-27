use crate::app::EditorMode;
use crate::gui::inspector::shell::{compose_pane_output, Inspector};
use crate::gui::text_input::committed_name_change;
use crate::shared::scene_ui::inspector::{
    is_scene_component_hidden_in_prefab, linked_prefab_instance_state_for_scene_inspector,
    InspectorContext, InspectorOutput,
};
use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::storage::editor_config;
use engine_core::worlds::*;
use ::widgets::InputCommit;

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
    let player_proxy = comp_type_name::<PlayerProxy>();

    assert!(is_scene_component_hidden_in_prefab(current_room));
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

#[test]
fn selecting_room_does_not_unhide_hidden_inspector() {
    let mut inspector = Inspector::new();
    inspector.set_rect(Rect::new(100.0, 0.0, 325.0, 400.0));
    editor_config::set_inspector_visible(false);

    inspector.select_room();

    assert!(!inspector.is_visible());
}

#[test]
fn selecting_entity_does_not_unhide_hidden_inspector() {
    let mut inspector = Inspector::new();
    inspector.set_rect(Rect::new(100.0, 0.0, 325.0, 400.0));
    editor_config::set_inspector_visible(false);

    inspector.select_entity(Entity(7));

    assert!(!inspector.is_visible());
}

#[test]
fn selecting_same_entity_reuses_existing_entity_inspector() {
    let mut inspector = Inspector::new();

    inspector.select_entity(Entity(7));
    let first_addr = inspector.entity_inspector_addr();

    inspector.select_entity(Entity(7));

    assert_eq!(inspector.selected_entity(), Some(Entity(7)));
    assert_eq!(inspector.entity_inspector_addr(), first_addr);
}

#[test]
fn hidden_inspector_only_hit_tests_the_strip() {
    let mut inspector = Inspector::new();
    inspector.set_rect(Rect::new(100.0, 0.0, 325.0, 400.0));
    editor_config::set_inspector_visible(false);

    let strip_rect = inspector.strip_rect();
    let strip_center = vec2(
        strip_rect.x + strip_rect.w / 2.0,
        strip_rect.y + strip_rect.h / 2.0,
    );

    assert!(inspector.hit_test_point(strip_center));
    assert!(!inspector.hit_test_point(vec2(110.0, strip_rect.y + 20.0)));
}

#[test]
fn visible_shell_composes_header_before_body() {
    let mut pane = ();
    let order = std::cell::RefCell::new(Vec::new());

    let _ = compose_pane_output(
        &mut pane,
        true,
        |_| {
            order.borrow_mut().push("body");
            InspectorOutput::default()
        },
        |_| {
            order.borrow_mut().push("header");
            InspectorOutput::default()
        },
    );

    assert_eq!(*order.borrow(), vec!["header", "body"]);
}

#[test]
fn collapsed_shell_skips_body_and_still_draws_header() {
    let mut pane = ();
    let order = std::cell::RefCell::new(Vec::new());

    let _ = compose_pane_output(
        &mut pane,
        false,
        |_| {
            order.borrow_mut().push("body");
            InspectorOutput::default()
        },
        |_| {
            order.borrow_mut().push("header");
            InspectorOutput::default()
        },
    );

    assert_eq!(*order.borrow(), vec!["header"]);
}
