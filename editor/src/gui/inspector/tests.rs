use crate::app::EditorMode;
use crate::gui::inspector::shell::Inspector;
use crate::gui::properties::PropertyModule;
use crate::gui::text_input::committed_name_change;
use crate::shared::scene_ui::inspector::{
    is_scene_component_hidden_in_prefab, linked_prefab_instance_state_for_scene_inspector,
    InspectorContent, InspectorContext, InspectorHostAction, InspectorOutput,
};
use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::prefab::create_prefab;
use engine_core::storage::editor_config;
use engine_core::worlds::*;
use ::widgets::{input_is_focused, request_focus, InputCommit, WidgetId};

fn compose_pane_output<T, FBody, FHeader>(
    pane: &mut T,
    visible: bool,
    draw_body: FBody,
    draw_header: FHeader,
) -> InspectorOutput
where
    FBody: FnOnce(&mut T) -> InspectorOutput,
    FHeader: FnOnce(&mut T) -> InspectorOutput,
{
    let mut output = draw_header(pane);
    let body_output = if visible {
        draw_body(pane)
    } else {
        InspectorOutput::default()
    };
    output.merge(body_output);
    output
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
        start_world_id: None,
        game_default_font: None,
        event_tags: Vec::new(),
        room_zone_tool_active: false,
        room_zones_visible: false,
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
        start_world_id: None,
        game_default_font: None,
        event_tags: Vec::new(),
        room_zone_tool_active: false,
        room_zones_visible: false,
    };

    assert_eq!(ctx.selected_create_parent, None);
}

#[test]
fn room_zone_visibility_output_merge_preserves_host_action() {
    let mut output = InspectorOutput::default();
    output.merge(InspectorOutput {
        host_action: Some(InspectorHostAction::ToggleRoomZoneVisibility),
        ..InspectorOutput::default()
    });

    assert_eq!(
        output.host_action,
        Some(InspectorHostAction::ToggleRoomZoneVisibility)
    );
}

#[test]
fn room_zone_visibility_context_carries_flag() {
    let ctx = InspectorContext {
        command_mode: EditorMode::Room(RoomId(1)),
        show_linked_prefab_metadata: true,
        hide_room_only_components: false,
        selected_create_parent: None,
        game_name: None,
        start_world_id: None,
        game_default_font: None,
        event_tags: Vec::new(),
        room_zone_tool_active: false,
        room_zones_visible: true,
    };

    assert!(ctx.room_zones_visible);
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
fn entity_inspector_cycles_through_selected_entities() {
    let mut inspector = Inspector::new();
    let first = Entity(7);
    let second = Entity(8);
    inspector.set_cycle_entities(vec![first, second]);
    inspector.select_entity(first);

    assert_eq!(inspector.cycle_selected_entity(true), Some(second));
    assert_eq!(inspector.selected_entity(), Some(second));

    assert_eq!(inspector.cycle_selected_entity(true), Some(first));
    assert_eq!(inspector.selected_entity(), Some(first));

    assert_eq!(inspector.cycle_selected_entity(false), Some(second));
    assert_eq!(inspector.selected_entity(), Some(second));
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

#[test]
fn inspector_output_merge_keeps_first_cycle_target_and_existing_fields() {
    let first_target = Entity(7);
    let second_target = Entity(8);
    let first_bounds = Rect::new(1.0, 2.0, 3.0, 4.0);
    let second_bounds = Rect::new(5.0, 6.0, 7.0, 8.0);
    let mut output = InspectorOutput {
        cycle_target: Some(first_target),
        expanded_module_rect: Some(first_bounds),
        open_prefab_picker: true,
        ..InspectorOutput::default()
    };

    output.merge(InspectorOutput {
        cycle_target: Some(second_target),
        expanded_module_rect: Some(second_bounds),
        delete_prefab: true,
        refresh_event_tags: true,
        ..InspectorOutput::default()
    });

    assert_eq!(output.cycle_target, Some(first_target));
    assert_eq!(output.expanded_module_rect, Some(first_bounds));
    assert!(output.open_prefab_picker);
    assert!(output.delete_prefab);
    assert!(output.refresh_event_tags);
}

fn inspector_context(command_mode: EditorMode) -> InspectorContext {
    InspectorContext {
        command_mode,
        show_linked_prefab_metadata: false,
        hide_room_only_components: false,
        selected_create_parent: None,
        game_name: None,
        start_world_id: None,
        game_default_font: None,
        event_tags: Vec::new(),
        room_zone_tool_active: false,
        room_zones_visible: false,
    }
}

#[test]
fn inspector_strip_disables_collapse_without_collapsible_modules() {
    let mut inspector = Inspector::new();
    inspector.set_rect(Rect::new(100.0, 0.0, 350.0, 400.0));

    let strip = inspector.action_strip_layout(&inspector_context(EditorMode::Prefab(PrefabId(1))));

    assert!(strip.collapse_disabled);
    assert!(strip.collapse_rect.w > 0.0);
    assert!(strip.cycle_previous_rect.is_none());
    assert!(strip.cycle_next_rect.is_none());
}

#[test]
fn inspector_strip_collapse_capability_tracks_pane_modules() {
    let mut inspector = Inspector::new();
    inspector.set_rect(Rect::new(100.0, 0.0, 350.0, 400.0));
    let context = inspector_context(EditorMode::Room(RoomId(1)));

    assert!(!InspectorContent::has_collapsible_modules(&inspector.game));
    assert!(InspectorContent::has_collapsible_modules(&inspector.world));
    assert!(InspectorContent::has_collapsible_modules(&inspector.room));
    assert!(InspectorContent::has_collapsible_modules(&inspector.tilemap));

    inspector.select_room();
    assert!(!inspector.action_strip_layout(&context).collapse_disabled);
    inspector.select_tilemap();
    assert!(!inspector.action_strip_layout(&context).collapse_disabled);
    inspector.select_entity(Entity(7));
    assert!(!inspector.action_strip_layout(&context).collapse_disabled);
}

#[test]
fn inspector_strip_fills_the_inspector_content_outline_with_a_square_collapse_control() {
    let mut inspector = Inspector::new();
    let inspector_rect = Rect::new(100.0, 20.0, 350.0, 400.0);
    inspector.set_rect(inspector_rect);
    inspector.select_room();

    let strip = inspector.action_strip_layout(&inspector_context(EditorMode::Room(RoomId(1))));

    assert_eq!(strip.strip_rect.x, inspector_rect.x);
    assert_eq!(strip.strip_rect.y, inspector.strip_rect().y);
    assert_eq!(strip.strip_rect.y, inspector.content_frame_rect().y);
    assert_eq!(
        inspector.content_frame_rect().h,
        inspector_rect.h
            - crate::gui::gui_constants::inspector::CONTENT_TOP_OFFSET
            - crate::gui::gui_constants::inspector::EDGE_GAP
    );
    assert_eq!(
        strip.strip_rect.w,
        inspector_rect.w
            - crate::gui::gui_constants::inspector::EDGE_GAP
            - crate::gui::gui_constants::inspector::STRIP_WIDTH
    );
    assert!(strip.strip_rect.h < crate::gui::gui_constants::BTN_HEIGHT);
    assert_eq!(strip.collapse_rect.w, strip.collapse_rect.h);
    assert_eq!(strip.collapse_rect.h, strip.strip_rect.h);
    assert_eq!(
        strip.collapse_rect.x + strip.collapse_rect.w,
        strip.strip_rect.x + strip.strip_rect.w
            - crate::gui::gui_constants::INSET
    );
    assert_eq!(
        inspector.strip_rect().y,
        strip.strip_rect.y
    );
}

#[test]
fn inspector_strip_cycle_controls_require_room_entity_candidates() {
    let mut inspector = Inspector::new();
    inspector.set_rect(Rect::new(100.0, 0.0, 350.0, 400.0));
    inspector.set_cycle_entities(vec![Entity(7), Entity(8)]);
    inspector.select_entity(Entity(7));

    let room_strip = inspector.action_strip_layout(&inspector_context(EditorMode::Room(RoomId(1))));
    assert!(room_strip.cycle_previous_rect.is_some());
    assert!(room_strip.cycle_next_rect.is_some());
    assert_eq!(
        room_strip.cycle_next_rect.unwrap().x
            + crate::gui::gui_constants::inspector::ACTION_BUTTON_SIZE
            + crate::gui::gui_constants::inspector::ACTION_BUTTON_GAP,
        room_strip.collapse_rect.x
    );
    assert!(room_strip.cycle_previous_rect.is_some_and(|previous| {
        previous.y >= inspector.rect.y + crate::gui::gui_constants::inspector::HEADER_HEIGHT
            && previous.y + previous.h <= room_strip.strip_rect.y + room_strip.strip_rect.h
    }));

    let prefab_strip =
        inspector.action_strip_layout(&inspector_context(EditorMode::Prefab(PrefabId(1))));
    assert!(prefab_strip.cycle_previous_rect.is_none());
    assert!(prefab_strip.cycle_next_rect.is_none());

    inspector.select_room();
    let room_pane_strip =
        inspector.action_strip_layout(&inspector_context(EditorMode::Room(RoomId(1))));
    assert!(room_pane_strip.cycle_previous_rect.is_none());
    assert!(room_pane_strip.cycle_next_rect.is_none());

    inspector.select_entity(Entity(7));
    inspector.set_cycle_entities(vec![Entity(7)]);
    let one_candidate_strip =
        inspector.action_strip_layout(&inspector_context(EditorMode::Room(RoomId(1))));
    assert!(one_candidate_strip.cycle_previous_rect.is_none());
    assert!(one_candidate_strip.cycle_next_rect.is_none());
}

#[test]
fn inspector_collapse_all_releases_focus_from_hidden_inputs() {
    let mut inspector = Inspector::new();
    inspector.select_room();
    request_focus(WidgetId(0x1A2B), true);

    assert!(input_is_focused());
    inspector.collapse_active_pane();
    assert!(!input_is_focused());
}

#[test]
fn inspector_collapse_action_only_collapses_active_room_pane() {
    let mut inspector = Inspector::new();
    inspector.select_room();
    for module in &mut inspector.room.modules {
        module.set_expanded(true);
    }
    inspector.world.navigation.set_expanded(true);

    inspector.collapse_active_pane();

    assert!(inspector
        .room
        .modules
        .iter()
        .all(|module| module.expansion_state() == Some(false)));
    assert_eq!(inspector.world.navigation.expansion_state(), Some(true));
}
