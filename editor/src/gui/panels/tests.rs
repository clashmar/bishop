use crate::app::Editor;
use crate::app::EditorMode;
use crate::gui::panels::hierarchy_panel::{
    clear_drag_on_mouse_release, layout_entity_tree, prune_dead_hierarchy_state,
    room_mode_prefab_library, sync_prefab_root_expansion, PrefabHierarchyHost, RoomHierarchyHost,
};
use crate::shared::scene_ui::hierarchy::SceneHierarchyHost;
use crate::room::room_editor::RoomEditor;
use engine_core::prelude::*;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::collections::HashSet;

#[test]
fn prune_dead_hierarchy_state_removes_deleted_entities() {
    let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("hierarchy_prune_dead");
    let mut stage = crate::prefab::PrefabStage::new(test_game.name());
    let live = stage
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Live".to_string()))
        .finish();
    let dead = stage
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Dead".to_string()))
        .finish();

    {
        let mut ctx = stage.ctx_mut();
        Ecs::remove_entity(&mut ctx, dead);
    }

    let mut expanded = HashSet::from([live, dead]);
    let mut dragging = Some(dead);

    prune_dead_hierarchy_state(&stage.ecs, &mut expanded, &mut dragging);

    assert_eq!(expanded, HashSet::from([live]));
    assert_eq!(dragging, None);
}

#[test]
fn layout_entity_tree_includes_children_when_root_is_expanded() {
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

    let mut y = 0.0;
    layout_entity_tree(root, &mut y, &HashSet::new(), &ecs);
    assert_eq!(y, 22.0);

    let mut expanded_y = 0.0;
    layout_entity_tree(root, &mut expanded_y, &HashSet::from([root]), &ecs);
    assert_eq!(expanded_y, 44.0);
}

#[test]
fn sync_prefab_root_expansion_expands_new_roots_once() {
    let root = Entity(1);
    let mut expanded = HashSet::new();
    let mut seen_roots = HashSet::new();

    sync_prefab_root_expansion(&[root], &mut expanded, &mut seen_roots);
    assert!(expanded.contains(&root));
    assert!(seen_roots.contains(&root));

    expanded.remove(&root);
    sync_prefab_root_expansion(&[root], &mut expanded, &mut seen_roots);
    assert!(!expanded.contains(&root));
}

#[test]
fn sync_prefab_root_expansion_expands_roots_when_they_first_appear() {
    let root_a = Entity(1);
    let root_b = Entity(2);
    let mut expanded = HashSet::new();
    let mut seen_roots = HashSet::new();

    sync_prefab_root_expansion(&[root_a], &mut expanded, &mut seen_roots);
    expanded.remove(&root_a);

    sync_prefab_root_expansion(&[root_a, root_b], &mut expanded, &mut seen_roots);

    assert!(!expanded.contains(&root_a));
    assert!(expanded.contains(&root_b));
}

#[test]
fn clear_drag_on_mouse_release_clears_prefab_drag_on_blank_space_release() {
    let dragged = Entity(7);
    let mut dragging = Some(dragged);

    clear_drag_on_mouse_release(&mut dragging, true);

    assert_eq!(dragging, None);
}

#[test]
fn room_mode_prefab_library_is_available_for_global_and_room_rows() {
    let mut editor = Editor {
        cur_room_id: Some(RoomId(1)),
        ..Default::default()
    };

    let prefab_library =
        room_mode_prefab_library(editor.cur_room_id, &editor.game.prefab_library).unwrap();
    assert!(std::ptr::eq(prefab_library, &editor.game.prefab_library));

    editor.cur_room_id = None;
    assert!(room_mode_prefab_library(editor.cur_room_id, &editor.game.prefab_library).is_none());
}

#[test]
fn room_hierarchy_host_toggles_selection_additively() {
    let entity = Entity(7);
    let mut room_editor = RoomEditor::new();
    let mut host = RoomHierarchyHost {
        room_editor: &mut room_editor,
        mode: EditorMode::Room(RoomId(1)),
        prefab_library: None,
    };

    host.select_entity(entity, false);
    assert!(host.is_selected(entity));

    host.select_entity(entity, true);
    assert!(!host.is_selected(entity));
    assert_eq!(host.room_editor.inspector.target, None);
}

#[test]
fn prefab_hierarchy_host_toggles_selection_additively() {
    let entity = Entity(9);
    let mut prefab_editor = crate::prefab::PrefabEditor::new(
        PrefabId(1),
        "Prefab".to_string(),
        None,
    );
    let mut host = PrefabHierarchyHost {
        prefab_editor: &mut prefab_editor,
        mode: EditorMode::Prefab(PrefabId(1)),
    };

    host.select_entity(entity, false);
    assert!(host.is_selected(entity));

    host.select_entity(entity, true);
    assert!(!host.is_selected(entity));
    assert_eq!(host.prefab_editor.inspector.target, None);
}
