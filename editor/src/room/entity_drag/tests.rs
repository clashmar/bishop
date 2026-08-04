use super::alt_copy::{enter_alt_copy_mode, exit_alt_copy_mode};
use super::box_selection::collect_box_selected_entities;
use super::drag_motion::{begin_entity_drag, finish_entity_drag, step_active_entity_drag};
use super::pointer_down::apply_click_selection;
use super::state::{EntityDragCommand, EntityDragState};
use crate::gui::inspector::collider_module::edit::{
    clear_collider_edit,
    collider_edit_entity,
    toggle_collider_edit,
};
use crate::editor_global::{apply_pending_commands, push_command, request_undo, with_editor};
use crate::room::room_editor::RoomEditor;
use crate::room::selection::topmost_entity_from_click_candidates;
use crate::test_utils::{EditorServicesGuard, TestGameFolder, game_fs_test_lock, make_room_editor};
use bishop::prelude::{Rect, Vec2, vec2};
use engine_core::assets::SpriteManager;
use engine_core::ecs::{
    Children,
    Ecs,
    Entity,
    Parent,
    Player,
    Transform,
    update_entity_position,
};
use engine_core::worlds::{RoomId, RoomLayer};

#[test]
fn topmost_entity_from_click_candidates_prefers_camera_then_highest_z() {
    let camera = Entity(1);
    let non_camera = Entity(2);
    let candidates = vec![(non_camera, 9, false), (camera, 0, true)];

    assert_eq!(topmost_entity_from_click_candidates(&candidates), Some(camera));
}

#[test]
fn topmost_entity_from_click_candidates_prefers_highest_z_for_non_cameras() {
    let low = Entity(1);
    let high = Entity(2);
    let candidates = vec![(low, 1, false), (high, 7, false)];

    assert_eq!(topmost_entity_from_click_candidates(&candidates), Some(high));
}

#[test]
fn collect_box_selected_entities_filters_room_and_layer() {
    let room_id = RoomId(7);
    let mut ecs = Ecs::default();
    let mut sprite_manager = SpriteManager::default();
    let inside = ecs
        .create_entity()
        .with(Transform {
            position: vec2(8.0, 8.0),
            ..Default::default()
        })
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    let wrong_layer = ecs
        .create_entity()
        .with(Transform {
            position: vec2(8.0, 8.0),
            ..Default::default()
        })
        .with_current_room_layer(room_id, RoomLayer::Back)
        .finish();

    let selected = collect_box_selected_entities(
        &ecs,
        room_id,
        RoomLayer::Front,
        Rect::new(0.0, 0.0, 32.0, 32.0),
        &mut sprite_manager,
        16.0,
    );

    assert!(selected.contains(&inside));
    assert!(!selected.contains(&wrong_layer));
}

#[test]
fn apply_click_selection_shift_click_toggles_membership_without_drag() {
    let mut editor = RoomEditor::new();
    let entity = Entity(7);
    editor.selected_entities.insert(entity);

    let should_start_drag = apply_click_selection(&mut editor, entity, true);

    assert!(!should_start_drag);
    assert!(!editor.selected_entities.contains(&entity));
}

#[test]
fn apply_click_selection_plain_click_replaces_selection_and_clears_edit_modes() {
    let mut editor = RoomEditor::new();
    let first = Entity(7);
    let second = Entity(8);
    clear_collider_edit(first);
    editor.set_selected_entity(Some(first));
    assert!(toggle_collider_edit(first));

    let should_start_drag = apply_click_selection(&mut editor, second, false);

    assert!(should_start_drag);
    assert_eq!(editor.single_selected_entity(), Some(second));
    assert_eq!(collider_edit_entity(), None);
}

#[test]
fn apply_click_selection_plain_click_on_selected_member_preserves_multi_selection() {
    let mut editor = RoomEditor::new();
    let first = Entity(7);
    let second = Entity(8);
    editor.selected_entities.insert(first);
    editor.selected_entities.insert(second);

    let should_start_drag = apply_click_selection(&mut editor, first, false);

    assert!(should_start_drag);
    assert!(editor.selected_entities.contains(&first));
    assert!(editor.selected_entities.contains(&second));
    assert_eq!(editor.selected_entities.len(), 2);
}

#[test]
fn begin_entity_drag_records_anchor_offset_and_initial_positions() {
    let mut ecs = Ecs::default();
    let anchor = ecs
        .create_entity()
        .with(Transform {
            position: vec2(10.0, 12.0),
            ..Default::default()
        })
        .finish();
    let follower = ecs
        .create_entity()
        .with(Transform {
            position: vec2(20.0, 12.0),
            ..Default::default()
        })
        .finish();
    let mut editor = RoomEditor::new();
    editor.selected_entities.insert(anchor);
    editor.selected_entities.insert(follower);

    begin_entity_drag(
        &mut editor.drag_state.entity_drag,
        &editor.selected_entities,
        anchor,
        &ecs,
        vec2(8.0, 10.0),
    );

    assert!(editor.drag_state.entity_drag.dragging);
    assert_eq!(editor.drag_state.entity_drag.anchor_entity, Some(anchor));
    assert_eq!(editor.drag_state.entity_drag.drag_offset, vec2(2.0, 2.0));
    assert_eq!(editor.drag_state.entity_drag.drag_start_positions.len(), 2);
    assert_eq!(
        editor.drag_state.entity_drag.drag_initial_start_positions,
        editor.drag_state.entity_drag.drag_start_positions,
    );
}

#[test]
fn step_active_entity_drag_moves_all_entities_by_anchor_delta() {
    let mut ecs = Ecs::default();
    let anchor = ecs
        .create_entity()
        .with(Transform {
            position: vec2(10.0, 10.0),
            ..Default::default()
        })
        .finish();
    let follower = ecs
        .create_entity()
        .with(Transform {
            position: vec2(20.0, 10.0),
            ..Default::default()
        })
        .finish();
    let mut drag = EntityDragState::default();
    drag.dragging = true;
    drag.anchor_entity = Some(anchor);
    drag.drag_offset = Vec2::ZERO;
    drag.drag_start_positions = vec![(anchor, vec2(10.0, 10.0)), (follower, vec2(20.0, 10.0))];
    drag.drag_initial_start_positions = drag.drag_start_positions.clone();

    let commit = step_active_entity_drag(&mut drag, &mut ecs, vec2(14.0, 16.0), false, false, 16.0);

    let anchor_position = match ecs.get::<Transform>(anchor) {
        Some(transform) => transform.position,
        None => panic!("expected anchor transform after drag"),
    };
    let follower_position = match ecs.get::<Transform>(follower) {
        Some(transform) => transform.position,
        None => panic!("expected follower transform after drag"),
    };
    assert_eq!(anchor_position, vec2(14.0, 16.0));
    assert_eq!(follower_position, vec2(24.0, 16.0));
    assert!(commit.is_none());
}

#[test]
fn finish_entity_drag_returns_move_many_when_multiple_entities_moved() {
    let mut ecs = Ecs::default();
    let first = ecs
        .create_entity()
        .with(Transform {
            position: vec2(15.0, 18.0),
            ..Default::default()
        })
        .finish();
    let second = ecs
        .create_entity()
        .with(Transform {
            position: vec2(25.0, 18.0),
            ..Default::default()
        })
        .finish();
    let mut drag = EntityDragState::default();
    drag.dragging = true;
    drag.drag_initial_start_positions = vec![
        (first, vec2(10.0, 10.0)),
        (second, vec2(20.0, 10.0)),
    ];

    let commit = finish_entity_drag(&mut drag, &ecs);

    assert_eq!(
        commit,
        Some(EntityDragCommand::MoveMany {
            moves: vec![
                (first, vec2(10.0, 10.0), vec2(15.0, 18.0)),
                (second, vec2(20.0, 10.0), vec2(25.0, 18.0)),
            ],
        }),
    );
    assert!(!drag.dragging);
}

#[test]
fn enter_alt_copy_mode_duplicates_selection_and_switches_anchor() {
    let room_id = RoomId(7);
    let mut ecs = Ecs::default();
    let anchor = ecs
        .create_entity()
        .with(Transform::default())
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    let mut editor = RoomEditor::new();
    editor.selected_entities.insert(anchor);
    begin_entity_drag(
        &mut editor.drag_state.entity_drag,
        &editor.selected_entities,
        anchor,
        &ecs,
        Vec2::ZERO,
    );

    let entered = enter_alt_copy_mode(&mut editor, &mut ecs, room_id);

    assert!(entered);
    assert!(editor.drag_state.entity_drag.alt_copy_mode);
    assert_eq!(editor.selected_entities.len(), 1);
    assert_ne!(editor.single_selected_entity(), Some(anchor));
}

#[test]
fn exit_alt_copy_mode_restores_original_selection_and_drag_positions() {
    let room_id = RoomId(7);
    let mut ecs = Ecs::default();
    let anchor = ecs
        .create_entity()
        .with(Transform {
            position: vec2(10.0, 10.0),
            ..Default::default()
        })
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    let mut editor = RoomEditor::new();
    editor.selected_entities.insert(anchor);
    begin_entity_drag(
        &mut editor.drag_state.entity_drag,
        &editor.selected_entities,
        anchor,
        &ecs,
        Vec2::ZERO,
    );
    assert!(enter_alt_copy_mode(&mut editor, &mut ecs, room_id));
    let duplicate = match editor.single_selected_entity() {
        Some(entity) => entity,
        None => panic!("expected duplicate selection after entering alt copy mode"),
    };
    update_entity_position(&mut ecs, duplicate, vec2(14.0, 18.0));

    let reverted = exit_alt_copy_mode(&mut editor, &mut ecs, vec2(14.0, 18.0));

    assert!(reverted);
    assert_eq!(editor.single_selected_entity(), Some(anchor));
    assert!(!editor.drag_state.entity_drag.alt_copy_mode);
    assert!(editor.drag_state.entity_drag.dragging);
    let anchor_position = match ecs.get::<Transform>(anchor) {
        Some(transform) => transform.position,
        None => panic!("expected original anchor transform after exiting alt copy mode"),
    };
    assert_eq!(anchor_position, vec2(14.0, 18.0));
}

#[test]
fn enter_alt_copy_mode_without_duplicates_keeps_normal_drag() {
    let room_id = RoomId(7);
    let mut ecs = Ecs::default();
    let anchor = ecs
        .create_entity()
        .with(Player::default())
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    let mut editor = RoomEditor::new();
    editor.selected_entities.insert(anchor);
    editor.drag_state.entity_drag.dragging = true;
    editor.drag_state.entity_drag.anchor_entity = Some(anchor);

    let entered = enter_alt_copy_mode(&mut editor, &mut ecs, room_id);

    assert!(!entered);
    assert!(!editor.drag_state.entity_drag.alt_copy_mode);
    assert_eq!(editor.single_selected_entity(), Some(anchor));
}

#[test]
fn alt_drag_copy_undo_preserves_original_hierarchy_and_removes_duplicates() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("alt_drag_copy_hierarchy_undo");
    let (mut editor, room_id) = make_room_editor(&test_game);

    let original_root = editor
        .game
        .ecs
        .create_entity()
        .with(Transform {
            position: vec2(16.0, 16.0),
            ..Default::default()
        })
        .with(Children::default())
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    let original_child = editor
        .game
        .ecs
        .create_entity()
        .with(Transform {
            position: vec2(24.0, 16.0),
            ..Default::default()
        })
        .with(Parent(original_root))
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    editor
        .game
        .ecs
        .get_mut::<Children>(original_root)
        .expect("root should have children component")
        .add(original_child);
    editor.room_editor.set_selected_entity(Some(original_root));

    let _guard = EditorServicesGuard::install(editor);

    let duplicated_root = with_editor(|editor| {
        editor
            .room_editor
            .duplicate_entities_for_drag(&mut editor.game.ecs, room_id)
            .into_iter()
            .next()
            .map(|(_, duplicate)| duplicate)
            .expect("expected duplicated root")
    });

    push_command(Box::new(crate::commands::room::AltDragCopyCmd::new(
        vec![duplicated_root],
        crate::app::EditorMode::Room(room_id),
    )));
    apply_pending_commands();
    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.game.ecs.get::<Transform>(duplicated_root).is_none());
        assert!(editor.game.ecs.get::<Transform>(original_root).is_some());
        assert!(editor.game.ecs.get::<Transform>(original_child).is_some());
        let children = editor
            .game
            .ecs
            .get::<Children>(original_root)
            .expect("original root should still have children");
        assert!(children.contains(original_child));
    });
}
