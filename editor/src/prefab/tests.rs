use crate::app::{Editor, EditorMode};
use crate::prefab::prefab_actions::PrefabEditorLaunch;
use crate::storage::editor_storage::create_new_game;
use engine_core::prelude::*;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};

fn make_editor_with_selected_entity(entity: Entity) -> Editor {
    let mut editor = Editor {
        mode: EditorMode::Room(RoomId(1)),
        ..Default::default()
    };
    editor.room_editor.selected_entities.insert(entity);
    editor
}

fn make_room_editor(test_game: &TestGameFolder) -> (Editor, RoomId) {
    let mut game = create_new_game(test_game.name().to_string());
    let cur_world_id = game.current_world_id;
    let room_id = game
        .current_world()
        .starting_room_id
        .expect("new game should have a starting room");
    game.get_world_mut(cur_world_id).current_room_id = Some(room_id);
    let editor = Editor {
        game,
        mode: EditorMode::Room(room_id),
        cur_world_id: Some(cur_world_id),
        cur_room_id: Some(room_id),
        ..Default::default()
    };

    (editor, room_id)
}

#[test]
fn prefab_editor_launch_prefers_root_component() {
    let mut editor = make_editor_with_selected_entity(Entity(1));
    editor.game.ecs.add_component_to_entity(
        Entity(1),
        PrefabInstanceRoot {
            prefab_id: PrefabId(10),
        },
    );
    editor.game.ecs.add_component_to_entity(
        Entity(1),
        PrefabInstanceNode {
            prefab_id: PrefabId(20),
            node_id: 7,
            root_entity: Entity(1),
        },
    );

    assert_eq!(
        editor.prefab_editor_launch(),
        PrefabEditorLaunch::OpenExisting(PrefabId(10))
    );
}

#[test]
fn prefab_editor_launch_uses_node_component_when_root_missing() {
    let mut editor = make_editor_with_selected_entity(Entity(1));
    editor.game.ecs.add_component_to_entity(
        Entity(1),
        PrefabInstanceNode {
            prefab_id: PrefabId(20),
            node_id: 7,
            root_entity: Entity(2),
        },
    );

    assert_eq!(
        editor.prefab_editor_launch(),
        PrefabEditorLaunch::OpenExisting(PrefabId(20))
    );
}

#[test]
fn prefab_editor_launch_prompts_for_selection_when_no_prefab_metadata_exists() {
    let editor = make_editor_with_selected_entity(Entity(1));

    assert_eq!(
        editor.prefab_editor_launch(),
        PrefabEditorLaunch::CaptureSelection(Entity(1))
    );
}

#[test]
fn prefab_editor_launch_opens_picker_outside_room_mode() {
    let editor = Editor {
        mode: EditorMode::Prefab(PrefabId(3)),
        ..Default::default()
    };

    assert_eq!(editor.prefab_editor_launch(), PrefabEditorLaunch::OpenPicker);
}

#[test]
fn prefab_editor_launch_opens_picker_when_room_has_no_selection() {
    let editor = Editor {
        mode: EditorMode::Room(RoomId(1)),
        ..Default::default()
    };

    assert_eq!(editor.prefab_editor_launch(), PrefabEditorLaunch::OpenPicker);
}

#[test]
fn prefab_editor_launch_opens_picker_when_room_has_multiple_selected_entities() {
    let mut editor = Editor {
        mode: EditorMode::Room(RoomId(1)),
        ..Default::default()
    };
    editor.room_editor.selected_entities.insert(Entity(1));
    editor.room_editor.selected_entities.insert(Entity(2));

    assert_eq!(editor.prefab_editor_launch(), PrefabEditorLaunch::OpenPicker);
}

#[test]
fn create_prefab_from_selection_relinks_selected_room_subtree() {
    let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_relink_selection");
    let (mut editor, room_id) = make_room_editor(&test_game);

    let root = editor
        .game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(48.0, 96.0),
            ..Default::default()
        })
        .with(CurrentRoom(room_id))
        .with(Name("Root".to_string()))
        .finish();
    let child = editor
        .game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(64.0, 112.0),
            ..Default::default()
        })
        .with(CurrentRoom(room_id))
        .with(Name("Child".to_string()))
        .finish();
    set_parent(&mut editor.game.ecs, child, root);
    editor.room_editor.set_selected_entity(Some(root));

    editor.create_prefab_from_selection(&(), root, "Crate".to_string());

    let linked_root = editor
        .room_editor
        .single_selected_entity()
        .expect("prefab relink should select replacement root");

    assert_ne!(linked_root, root);
    assert_eq!(editor.mode, EditorMode::Prefab(PrefabId(1)));
    assert!(!editor.game.ecs.has::<Transform>(root));
    assert!(!editor.game.ecs.has::<Transform>(child));
    assert!(editor.game.ecs.has::<Transform>(linked_root));
    assert_eq!(
        editor
            .game
            .ecs
            .get::<PrefabInstanceRoot>(linked_root)
            .map(|root| root.prefab_id),
        Some(PrefabId(1))
    );
    assert_eq!(
        editor
            .game
            .ecs
            .get::<CurrentRoom>(linked_root)
            .map(|room| room.0),
        Some(room_id)
    );
    assert_eq!(
        editor
            .game
            .ecs
            .get::<Transform>(linked_root)
            .map(|transform| transform.position),
        Some(Vec2::new(48.0, 96.0))
    );

    let linked_nodes = editor
        .game
        .ecs
        .get_store::<PrefabInstanceNode>()
        .data
        .iter()
        .filter(|(_, node)| node.prefab_id == PrefabId(1) && node.root_entity == linked_root)
        .count();
    assert_eq!(linked_nodes, 2);

    editor.mode = EditorMode::Room(room_id);
    assert_eq!(
        editor.prefab_editor_launch(),
        PrefabEditorLaunch::OpenExisting(PrefabId(1))
    );
}

#[test]
fn create_prefab_from_selection_preserves_external_parent() {
    let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_relink_parent");
    let (mut editor, room_id) = make_room_editor(&test_game);

    let container = editor
        .game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(12.0, 24.0),
            ..Default::default()
        })
        .with(CurrentRoom(room_id))
        .with(Name("Container".to_string()))
        .finish();
    let root = editor
        .game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(80.0, 120.0),
            ..Default::default()
        })
        .with(CurrentRoom(room_id))
        .with(Name("Root".to_string()))
        .finish();
    set_parent(&mut editor.game.ecs, root, container);
    editor.room_editor.set_selected_entity(Some(root));

    editor.create_prefab_from_selection(&(), root, "Crate".to_string());

    let linked_root = editor
        .room_editor
        .single_selected_entity()
        .expect("prefab relink should select replacement root");

    assert_eq!(get_parent(&editor.game.ecs, linked_root), Some(container));
    assert_eq!(
        editor
            .game
            .ecs
            .get::<PrefabInstanceRoot>(linked_root)
            .map(|root| root.prefab_id),
        Some(PrefabId(1))
    );
}
