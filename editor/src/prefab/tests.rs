use crate::app::{Editor, EditorMode, PendingPrefabTransition, PrefabTransitionPrompt};
use crate::commands::editor_command_manager::EditorCommand;
use crate::commands::scene::{
    ApplyInstanceToPrefabCmd, DeleteEntityCmd, RevertPrefabInstanceCmd, UnlinkPrefabInstanceCmd,
    UpdateComponentCmd,
};
use crate::editor_global::{
    apply_pending_commands, push_command, request_redo, request_undo, reset_services, set_editor,
    with_command_manager, with_editor, EDITOR_SERVICES,
};
use crate::gui::prompts::{DirtyPrefabExitPromptResult, EmptyPrefabExitPromptResult};
use crate::prefab::prefab_actions::PrefabEditorLaunch;
use crate::prefab::prefab_editor::{PrefabRoomSyncState, StagedPrefabState};
use crate::prefab::{PrefabEditor, PrefabStage};
use crate::storage::editor_storage::create_new_game;
use crate::storage::editor_storage::save_game;
use engine_core::prelude::*;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::path::PathBuf;

struct EditorServicesGuard;

impl EditorServicesGuard {
    fn install(editor: Editor) -> Self {
        reset_services();
        set_editor(editor);
        Self
    }
}

impl Drop for EditorServicesGuard {
    fn drop(&mut self) {
        EDITOR_SERVICES.with(|services| {
            *services.editor.borrow_mut() = None;
        });
        reset_services();
    }
}

fn make_editor_with_selected_entity(entity: Entity) -> Editor {
    let mut editor = Editor {
        mode: EditorMode::Room(RoomId(1)),
        ..Default::default()
    };
    editor.room_editor.selected_entities.insert(entity);
    editor
}

fn make_room_editor(test_game: &TestGameFolder) -> (Editor, RoomId) {
    set_game_name(test_game.name());
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

fn linked_root_entities(ecs: &Ecs, prefab_id: PrefabId) -> Vec<Entity> {
    ecs.get_store::<PrefabInstanceRoot>()
        .data
        .iter()
        .filter_map(|(&entity, root)| (root.prefab_id == prefab_id).then_some(entity))
        .collect()
}

fn linked_instance_node_count(ecs: &Ecs, prefab_id: PrefabId) -> usize {
    ecs.get_store::<PrefabInstanceNode>()
        .data
        .values()
        .filter(|node| node.prefab_id == prefab_id)
        .count()
}

fn make_prefab_session_editor(test_game: &TestGameFolder) -> (Editor, RoomId, PrefabId, Entity) {
    let (mut editor, room_id) = make_room_editor(test_game);

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
    editor.room_editor.set_selected_entity(Some(root));
    editor.create_prefab_from_selection(&(), root, "Crate".to_string());

    (editor, room_id, PrefabId(1), root)
}

fn save_test_prefab(test_game: &TestGameFolder, prefab_id: PrefabId, name: &str) -> PrefabAsset {
    let prefab = create_prefab(prefab_id, name.to_string());
    save_prefab(test_game.name(), &prefab).unwrap();
    prefab
}

fn write_invalid_prefab(_test_game: &TestGameFolder, file_name: &str) -> PathBuf {
    let path = prefabs_folder().join(file_name);
    std::fs::write(&path, "not valid ron").unwrap();
    path
}

fn add_prefab_child_entity(editor: &mut Editor, position: Vec2) -> Entity {
    let prefab_editor = editor
        .prefab_editor
        .as_mut()
        .expect("prefab editor should exist");
    let root = prefab_editor.root_entity.expect("prefab root should exist");
    let child = prefab_editor.create_prefab_entity(
        &mut editor
            .prefab_stage
            .as_mut()
            .expect("prefab stage should exist")
            .ecs,
        Some(root),
    );
    editor
        .prefab_stage
        .as_mut()
        .expect("prefab stage should exist")
        .ecs
        .get_mut::<Transform>(child)
        .expect("prefab child should have transform")
        .position = position;
    child
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

    assert_eq!(
        editor.prefab_editor_launch(),
        PrefabEditorLaunch::OpenPicker
    );
}

#[test]
fn prefab_editor_launch_opens_picker_when_room_has_no_selection() {
    let editor = Editor {
        mode: EditorMode::Room(RoomId(1)),
        ..Default::default()
    };

    assert_eq!(
        editor.prefab_editor_launch(),
        PrefabEditorLaunch::OpenPicker
    );
}

#[test]
fn prefab_editor_launch_opens_picker_when_room_has_multiple_selected_entities() {
    let mut editor = Editor {
        mode: EditorMode::Room(RoomId(1)),
        ..Default::default()
    };
    editor.room_editor.selected_entities.insert(Entity(1));
    editor.room_editor.selected_entities.insert(Entity(2));

    assert_eq!(
        editor.prefab_editor_launch(),
        PrefabEditorLaunch::OpenPicker
    );
}

#[test]
fn create_prefab_from_selection_relinks_selected_room_subtree() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
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
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
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

#[test]
fn prefab_stage_uses_project_sprite_paths_without_room_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_stage_game");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    game.asset_manager
        .sprite_id_to_path
        .insert(SpriteId(7), PathBuf::from("sprites/cat.png"));
    save_game(&game).unwrap();

    let mut stage = PrefabStage::new(test_game.name());
    let prefab_ctx = stage.ctx_mut();

    assert_eq!(
        prefab_ctx
            .asset_manager
            .sprite_id_to_path
            .get(&SpriteId(7))
            .cloned(),
        Some(PathBuf::from("sprites/cat.png"))
    );
    assert!(prefab_ctx.ecs.get_store::<RoomCamera>().data.is_empty());
    assert!(prefab_ctx.ecs.get_store::<CurrentRoom>().data.is_empty());
    assert!(prefab_ctx.world.is_none());
}

#[test]
fn editor_services_guard_clears_global_editor_on_drop() {
    {
        let _guard = EditorServicesGuard::install(Editor::default());
        EDITOR_SERVICES.with(|services| {
            assert!(services.editor.borrow().is_some());
        });
    }

    EDITOR_SERVICES.with(|services| {
        assert!(services.editor.borrow().is_none());
    });
}

#[test]
fn creating_entity_replaces_stale_root_with_new_root() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_stale_root");
    set_game_name(test_game.name());
    let mut editor = PrefabEditor::new(
        PrefabId(1),
        "Prefab".to_string(),
        StagedPrefabState::Empty,
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::Empty,
            linked_instance_snapshots: Vec::new(),
        },
    );
    let mut stage = PrefabStage::new(test_game.name());

    let stale_root = stage
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Old Root".to_string()))
        .finish();
    editor.root_entity = Some(stale_root);
    editor.set_selected_entity(Some(stale_root));

    {
        let mut ctx = stage.ctx_mut();
        Ecs::remove_entity(&mut ctx, stale_root);
    }

    let new_entity = editor.create_prefab_entity(&mut stage.ecs, None);

    assert_eq!(editor.root_entity, Some(new_entity));
    assert_eq!(get_parent(&stage.ecs, new_entity), None);
}

#[test]
fn prefab_child_keyboard_move_updates_position_and_supports_undo_redo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_child_keyboard_move");
    let (mut editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let child = add_prefab_child_entity(&mut editor, Vec2::new(12.0, 18.0));
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_mut().unwrap();
        let prefab_editor = editor.prefab_editor.as_mut().unwrap();
        prefab_editor.set_selected_entity(Some(child));
        prefab_editor.move_selected_entities_by(&mut stage.ecs, Vec2::new(1.0, 0.0));
    });
    apply_pending_commands();

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_mut().unwrap();
        assert_eq!(
            stage
                .ecs
                .get::<Transform>(child)
                .map(|transform| transform.position),
            Some(Vec2::new(13.0, 18.0))
        );

        let staged_prefab = editor
            .prefab_editor
            .as_mut()
            .unwrap()
            .staged_prefab_state(&mut stage.ctx_mut());
        let StagedPrefabState::PrefabAsset(prefab) = staged_prefab else {
            panic!("expected prefab asset state");
        };
        let child_node = prefab
            .nodes
            .iter()
            .find(|node| node.node_id != prefab.root_node_id)
            .expect("child node should exist");
        let child_transform = child_node
            .components
            .iter()
            .find(|component| component.type_name == Transform::TYPE_NAME)
            .expect("child transform should be captured");
        assert!(child_transform.ron.contains("position:(13.0,18.0)"));
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_ref().unwrap();
        assert_eq!(
            stage
                .ecs
                .get::<Transform>(child)
                .map(|transform| transform.position),
            Some(Vec2::new(12.0, 18.0))
        );
    });

    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_ref().unwrap();
        assert_eq!(
            stage
                .ecs
                .get::<Transform>(child)
                .map(|transform| transform.position),
            Some(Vec2::new(13.0, 18.0))
        );
        assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
    });
}

#[test]
fn prefab_root_keyboard_move_is_ignored() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_root_keyboard_move_ignored");
    let (editor, _, _, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    let root_before = with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        let pos = editor
            .prefab_stage
            .as_ref()
            .unwrap()
            .ecs
            .get::<Transform>(root)
            .expect("prefab root should have transform")
            .position;
        (root, pos)
    });

    with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        let stage = editor.prefab_stage.as_mut().unwrap();
        let prefab_editor = editor.prefab_editor.as_mut().unwrap();
        prefab_editor.set_selected_entity(Some(root));
        prefab_editor.move_selected_entities_by(&mut stage.ecs, Vec2::new(1.0, 0.0));
    });
    apply_pending_commands();

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_ref().unwrap();
        assert_eq!(
            stage
                .ecs
                .get::<Transform>(root_before.0)
                .map(|transform| transform.position),
            Some(root_before.1)
        );
    });
    with_command_manager(|manager| {
        assert_eq!(manager.undo_stack_len(), 0);
        assert_eq!(manager.redo_stack_len(), 0);
        assert_eq!(manager.pending_len(), 0);
    });
}

#[test]
fn deleting_prefab_root_clears_root_and_selection_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_delete_root");
    set_game_name(test_game.name());
    let mut editor = Editor {
        mode: EditorMode::Prefab(PrefabId(9)),
        prefab_editor: Some(PrefabEditor::new(
            PrefabId(9),
            "Prefab".to_string(),
            StagedPrefabState::Empty,
            PrefabRoomSyncState {
                staged_prefab: StagedPrefabState::Empty,
                linked_instance_snapshots: Vec::new(),
            },
        )),
        prefab_stage: Some(PrefabStage::new(test_game.name())),
        ..Default::default()
    };

    let root = editor
        .prefab_stage
        .as_mut()
        .unwrap()
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Root".to_string()))
        .finish();
    let child = editor
        .prefab_stage
        .as_mut()
        .unwrap()
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Child".to_string()))
        .finish();
    set_parent(&mut editor.prefab_stage.as_mut().unwrap().ecs, child, root);

    let prefab_editor = editor.prefab_editor.as_mut().unwrap();
    prefab_editor.root_entity = Some(root);
    prefab_editor.selected_entities.insert(root);
    prefab_editor.selected_entities.insert(child);

    let _guard = EditorServicesGuard::install(editor);

    let mut cmd = DeleteEntityCmd::new(root, EditorMode::Prefab(PrefabId(9)));
    cmd.execute();

    with_editor(|editor| {
        let prefab_editor = editor.prefab_editor.as_ref().unwrap();
        assert_eq!(prefab_editor.root_entity, None);
        assert!(prefab_editor.selected_entities.is_empty());
    });
}

#[test]
fn staged_prefab_edits_preview_sync_to_linked_room_instances() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_preview_sync");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let prefab_editor = editor.prefab_editor.as_mut().unwrap();
        let root = prefab_editor.root_entity.expect("prefab root should exist");
        let child = prefab_editor
            .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));
        editor
            .prefab_stage
            .as_mut()
            .unwrap()
            .ecs
            .add_component_to_entity(child, Name("Preview Child".to_string()));
        editor.reconcile_active_prefab_room_preview();
    });

    with_editor(|editor| {
        let linked_roots = linked_root_entities(&editor.game.ecs, prefab_id);
        assert_eq!(linked_roots.len(), 1);
        assert_eq!(linked_instance_node_count(&editor.game.ecs, prefab_id), 2);
        assert_eq!(
            editor
                .game
                .ecs
                .get::<CurrentRoom>(linked_roots[0])
                .map(|room| room.0),
            Some(room_id)
        );
    });
}

#[test]
fn empty_prefab_preview_delete_and_undo_restore_room_instances() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_empty_preview_undo");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(editor.prefab_editor.as_ref().unwrap().root_entity.is_some());
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 1);
    });
}

#[test]
fn saving_empty_prefab_delete_supports_undo_and_redo_preview_sync() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_empty_save_undo_redo");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        editor.confirm_empty_prefab_save_delete();
        assert!(!editor.game.prefab_library.prefabs.contains_key(&prefab_id));
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(editor.prefab_editor.as_ref().unwrap().root_entity.is_some());
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 1);
    });

    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(editor.prefab_editor.as_ref().unwrap().root_entity.is_none());
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
    });
}

#[test]
fn discarding_empty_prefab_exit_restores_committed_room_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_empty_exit_discard");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
        assert_eq!(
            editor.request_prefab_transition(PendingPrefabTransition::Exit),
            PrefabTransitionPrompt::Empty
        );
        editor.confirm_empty_prefab_transition(EmptyPrefabExitPromptResult::DiscardChanges);
        assert_eq!(editor.mode, EditorMode::Room(room_id));
        assert!(editor.prefab_editor.is_none());
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 1);
    });
}

#[test]
fn clean_prefab_transition_opens_requested_prefab_without_changing_return_mode() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_clean_switch");
    let (mut editor, room_id, _, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
    editor
        .game
        .prefab_library
        .prefabs
        .insert(second_prefab.id, second_prefab.clone());

    assert_eq!(
        editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(second_prefab.id)),
        PrefabTransitionPrompt::None
    );
    assert_eq!(editor.mode, EditorMode::Prefab(second_prefab.id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(
        editor
            .prefab_editor
            .as_ref()
            .map(|prefab| prefab.prefab_name.as_str()),
        Some("Barrel")
    );
}

#[test]
fn requesting_prefab_transition_from_asset_loads_prefab_into_library() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_transition_asset_load");
    let (mut editor, room_id, _, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");

    assert!(!editor
        .game
        .prefab_library
        .prefabs
        .contains_key(&second_prefab.id));
    assert_eq!(
        editor.request_prefab_transition_to_asset(second_prefab.clone()),
        PrefabTransitionPrompt::None
    );
    assert_eq!(editor.mode, EditorMode::Prefab(second_prefab.id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(
        editor.game.prefab_library.prefabs.get(&second_prefab.id),
        Some(&second_prefab)
    );
}

#[test]
fn requesting_prefab_transition_from_file_path_marks_dirty_session_pending() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_transition_file_dirty");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    let result = editor.request_prefab_transition_to_path(&prefabs_folder().join("2.ron"));

    assert_eq!(result.unwrap(), PrefabTransitionPrompt::Dirty);
    assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(
        editor.pending_prefab_transition,
        Some(PendingPrefabTransition::OpenExisting(second_prefab.id))
    );
    assert_eq!(
        editor.game.prefab_library.prefabs.get(&second_prefab.id),
        Some(&second_prefab)
    );
}

#[test]
fn requesting_prefab_transition_from_invalid_file_path_returns_error() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_transition_file_invalid");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let invalid_path = write_invalid_prefab(&test_game, "broken.ron");

    let error = editor
        .request_prefab_transition_to_path(&invalid_path)
        .unwrap_err();

    assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(editor.pending_prefab_transition, None);
    assert!(error.to_string().contains("Could not parse prefab"));
}

#[test]
fn dirty_prefab_transition_save_switches_and_persists_changes() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_dirty_switch_save");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
    editor
        .game
        .prefab_library
        .prefabs
        .insert(second_prefab.id, second_prefab.clone());

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    assert_eq!(
        editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(second_prefab.id)),
        PrefabTransitionPrompt::Dirty
    );
    assert_eq!(
        editor.pending_prefab_transition,
        Some(PendingPrefabTransition::OpenExisting(second_prefab.id))
    );

    editor.confirm_dirty_prefab_transition(DirtyPrefabExitPromptResult::SaveAndSync);

    assert_eq!(editor.mode, EditorMode::Prefab(second_prefab.id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(
        editor
            .game
            .prefab_library
            .prefabs
            .get(&prefab_id)
            .map(|prefab| prefab.nodes.len()),
        Some(2)
    );
    assert_eq!(editor.pending_prefab_transition, None);
}

#[test]
fn dirty_prefab_transition_cancel_keeps_current_prefab_open() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_dirty_switch_cancel");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
    editor
        .game
        .prefab_library
        .prefabs
        .insert(second_prefab.id, second_prefab);

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    assert_eq!(
        editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(PrefabId(2))),
        PrefabTransitionPrompt::Dirty
    );

    editor.confirm_dirty_prefab_transition(DirtyPrefabExitPromptResult::Cancel);

    assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(editor.pending_prefab_transition, None);
    assert!(!editor.active_prefab_is_clean());
}

#[test]
fn empty_prefab_transition_delete_switches_to_requested_prefab() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_empty_switch_delete");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
        editor
            .game
            .prefab_library
            .prefabs
            .insert(second_prefab.id, second_prefab);

        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert_eq!(
            editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(PrefabId(2))),
            PrefabTransitionPrompt::Empty
        );
        editor.confirm_empty_prefab_transition(EmptyPrefabExitPromptResult::DeletePrefab);

        assert_eq!(editor.mode, EditorMode::Prefab(PrefabId(2)));
        assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
        assert!(!editor.game.prefab_library.prefabs.contains_key(&prefab_id));
        assert_eq!(editor.pending_prefab_transition, None);
    });
}

#[test]
fn blank_prefab_transition_does_not_create_asset_until_confirmed() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_new_switch_cancel");
    let (mut editor, _, _, _) = make_prefab_session_editor(&test_game);

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    assert_eq!(
        editor
            .request_prefab_transition(PendingPrefabTransition::CreateBlank("Fresh".to_string(),)),
        PrefabTransitionPrompt::Dirty
    );
    assert!(editor
        .game
        .prefab_library
        .prefabs
        .values()
        .all(|prefab| prefab.name != "Fresh"));

    editor.confirm_dirty_prefab_transition(DirtyPrefabExitPromptResult::Cancel);

    assert!(editor
        .game
        .prefab_library
        .prefabs
        .values()
        .all(|prefab| prefab.name != "Fresh"));
}

#[test]
fn saving_new_prefab_session_marks_prefab_clean_for_exit() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_new_session_clean_after_save");
    set_game_name(test_game.name());
    let prefab = create_prefab(PrefabId(1), "Crate".to_string());
    let (mut prefab_editor, mut prefab_stage) = PrefabEditor::open_existing(
        test_game.name(),
        prefab.clone(),
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::PrefabAsset(prefab),
            linked_instance_snapshots: Vec::new(),
        },
    );

    let entity = prefab_editor.create_prefab_entity(&mut prefab_stage.ecs, None);
    prefab_editor.set_selected_entity(Some(entity));

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        mode: EditorMode::Prefab(PrefabId(1)),
        prefab_editor: Some(prefab_editor),
        prefab_stage: Some(prefab_stage),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let staged_state = editor.active_prefab_staged_state();
        assert!(matches!(
            staged_state,
            Some(StagedPrefabState::PrefabAsset(_))
        ));
        editor.commit_prefab_asset_save(match staged_state {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        });
        assert!(editor.active_prefab_is_clean());
    });
}

#[test]
fn unlink_prefab_instance_command_clears_prefab_components_and_restores_on_undo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_unlink_instance");
    set_game_name(test_game.name());
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    editor.close_active_prefab_editor();
    let linked_root = linked_root_entities(&editor.game.ecs, prefab_id)[0];

    let _services = EditorServicesGuard::install(editor);

    push_command(Box::new(UnlinkPrefabInstanceCmd::new(
        linked_root,
        EditorMode::Room(room_id),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.game.ecs.has::<Transform>(linked_root));
        assert!(!editor.game.ecs.has::<PrefabInstanceRoot>(linked_root));
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 0);
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 1);
        assert_eq!(
            editor
                .game
                .ecs
                .get::<PrefabInstanceRoot>(linked_root)
                .map(|root| root.prefab_id),
            Some(prefab_id)
        );
    });
}

#[test]
fn room_component_edits_write_prefab_overrides_for_linked_instances() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_component_override_tracking");
    set_game_name(test_game.name());
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    editor.close_active_prefab_editor();

    let linked_root = linked_root_entities(&editor.game.ecs, prefab_id)[0];
    let old_ron = ron::to_string(
        &editor
            .game
            .ecs
            .get::<Name>(linked_root)
            .expect("linked instance should have a name"),
    )
    .expect("name should serialize");

    let _services = EditorServicesGuard::install(editor);

    push_command(Box::new(UpdateComponentCmd::new(
        linked_root,
        EditorMode::Room(room_id),
        Name::TYPE_NAME,
        old_ron,
        "(\"Edited Root\")".to_string(),
        Default::default(),
        Default::default(),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let overrides = editor
            .game
            .ecs
            .get::<PrefabOverrides>(linked_root)
            .expect("linked instance edit should create prefab overrides");
        assert!(overrides
            .modified_components
            .iter()
            .any(|type_name| type_name == Name::TYPE_NAME));
    });
}

#[test]
fn apply_instance_to_prefab_command_updates_other_linked_instances_and_supports_undo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_apply_instance_to_prefab");
    set_game_name(test_game.name());
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    editor.close_active_prefab_editor();

    let linked_root = linked_root_entities(&editor.game.ecs, prefab_id)[0];
    let clean_sibling_root = {
        let prefab = editor
            .game
            .prefab_library
            .prefabs
            .get(&prefab_id)
            .cloned()
            .expect("prefab should exist");
        let mut ctx = editor.game.ctx_mut();
        let mut services = ctx.services_ctx_mut();
        instantiate_prefab(
            &mut services,
            &prefab,
            Vec2::new(160.0, 96.0),
            Some(room_id),
        )
    };
    let overridden_sibling_root = {
        let prefab = editor
            .game
            .prefab_library
            .prefabs
            .get(&prefab_id)
            .cloned()
            .expect("prefab should exist");
        let mut ctx = editor.game.ctx_mut();
        let mut services = ctx.services_ctx_mut();
        instantiate_prefab(
            &mut services,
            &prefab,
            Vec2::new(256.0, 96.0),
            Some(room_id),
        )
    };
    editor
        .game
        .ecs
        .get_mut::<Name>(linked_root)
        .expect("linked root should have a name")
        .0 = "Edited Root".to_string();
    editor
        .game
        .ecs
        .get_mut::<Name>(overridden_sibling_root)
        .expect("overridden sibling should have a name")
        .0 = "Locally Tweaked Child".to_string();
    editor.game.ecs.add_component_to_entity(
        overridden_sibling_root,
        PrefabOverrides {
            modified_components: vec![Name::TYPE_NAME.to_string()],
            ..Default::default()
        },
    );

    let _services = EditorServicesGuard::install(editor);

    push_command(Box::new(ApplyInstanceToPrefabCmd::new(
        linked_root,
        EditorMode::Room(room_id),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let prefab = editor
            .game
            .prefab_library
            .prefabs
            .get(&prefab_id)
            .expect("prefab should be updated");
        let prefab_root = prefab
            .nodes
            .iter()
            .find(|node| node.node_id == prefab.root_node_id)
            .expect("prefab root node should exist");
        assert!(prefab_root
            .components
            .iter()
            .any(|component| component.type_name == Name::TYPE_NAME
                && component.ron.contains("Edited Root")));
        assert_eq!(
            editor
                .game
                .ecs
                .get::<Name>(clean_sibling_root)
                .map(|name| name.0.as_str()),
            Some("Edited Root")
        );
        assert_eq!(
            editor
                .game
                .ecs
                .get::<Name>(overridden_sibling_root)
                .map(|name| name.0.as_str()),
            Some("Locally Tweaked Child")
        );
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(
            editor
                .game
                .ecs
                .get::<Name>(linked_root)
                .map(|name| name.0.as_str()),
            Some("Edited Root")
        );
    });
}

#[test]
fn revert_instance_to_prefab_command_clears_overrides_and_restores_prefab_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_revert_instance_to_prefab");
    set_game_name(test_game.name());
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    editor.close_active_prefab_editor();

    let linked_root = linked_root_entities(&editor.game.ecs, prefab_id)[0];
    editor
        .game
        .ecs
        .get_mut::<Name>(linked_root)
        .expect("linked root should have name")
        .0 = "Edited Root".to_string();
    editor.game.ecs.add_component_to_entity(
        linked_root,
        PrefabOverrides {
            modified_components: vec![Name::TYPE_NAME.to_string()],
            ..Default::default()
        },
    );
    editor.room_editor.set_selected_entity(Some(linked_root));

    let _services = EditorServicesGuard::install(editor);

    push_command(Box::new(RevertPrefabInstanceCmd::new(
        linked_root,
        EditorMode::Room(room_id),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(
            editor
                .game
                .ecs
                .get::<Name>(linked_root)
                .map(|name| name.0.as_str()),
            Some("Root")
        );
        assert!(!editor.game.ecs.has::<PrefabOverrides>(linked_root));
        assert_eq!(
            editor.room_editor.single_selected_entity(),
            Some(linked_root)
        );
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(
            editor
                .game
                .ecs
                .get::<Name>(linked_root)
                .map(|name| name.0.as_str()),
            Some("Edited Root")
        );
        assert!(editor.game.ecs.has::<PrefabOverrides>(linked_root));
    });
}
