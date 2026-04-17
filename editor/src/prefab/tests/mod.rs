pub(super) use crate::app::{Editor, EditorMode, PendingPrefabTransition, PrefabTransitionPrompt};
pub(super) use crate::commands::scene::DeleteEntityCmd;
pub(super) use crate::editor_global::{
    apply_pending_commands, push_command, request_redo, request_undo, reset_services, set_editor,
    with_command_manager, with_editor, EDITOR_SERVICES,
};
pub(super) use crate::gui::prompts::{DirtyPrefabExitPromptResult, EmptyPrefabExitPromptResult};
pub(super) use crate::prefab::prefab_editor::actions::PrefabEditorLaunch;
pub(super) use crate::prefab::prefab_editor::{
    PrefabEditor, PrefabRoomSyncState, PrefabStage, StagedPrefabState,
};
pub(super) use crate::prefab::BLANK_PREFAB_ID;
pub(super) use crate::room::room_editor::{RoomEditorMode, RoomSceneSubMode};
pub(super) use crate::storage::editor_storage::{
    create_new_game, load_game_by_name, load_prefab_palette_state, save_game,
};
pub(super) use engine_core::prelude::*;
pub(super) use engine_core::storage::path_utils::{game_folder, sanitise_name};
pub(super) use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
pub(super) use std::path::PathBuf;

mod movement_tests;
mod blank_prefab_session_tests;
mod prefab_actions_tests;
mod prefab_editor_tests;
mod delete_prefab_cmd_tests;
mod prefab_room_sync_tests;
mod prefab_save_tests;
mod prefab_transition_tests;
mod selection_tests;

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

fn saved_prefab_path(prefab: &PrefabAsset) -> PathBuf {
    prefabs_folder().join(format!("{}.ron", sanitise_name(&prefab.name)))
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
