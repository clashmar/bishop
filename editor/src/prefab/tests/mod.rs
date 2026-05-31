pub(super) use crate::app::{Editor, EditorMode};
pub(super) use crate::commands::scene::DeleteEntityCmd;
pub(super) use crate::editor_global::{
    apply_pending_commands, push_command, request_redo, request_undo, with_command_manager,
    with_editor, EDITOR_SERVICES,
};
pub(super) use crate::gui::prompts::{DirtyPrefabExitPromptResult, EmptyPrefabExitPromptResult};
pub(super) use crate::prefab::prefab_editor::actions::PrefabEditorLaunch;
pub(super) use crate::prefab::prefab_editor::{
    PrefabEditor, PrefabRoomSyncState, PrefabStage, StagedPrefabState,
};
pub(super) use crate::prefab::BLANK_PREFAB_ID;
pub(super) use crate::prefab::{PendingPrefabTransition, PrefabTransitionPrompt};
pub(super) use crate::room::room_editor::{RoomEditorMode, RoomSceneSubMode};
pub(super) use crate::storage::editor_storage::{
    create_new_game, load_game_by_name, load_prefab_palette_state, save_game,
};
pub(super) use crate::test_utils::{
    game_fs_test_lock, install_prefab_save_picker_result, linked_root_entities,
    make_prefab_session_editor, make_room_editor, EditorServicesGuard, TestGameFolder,
};
pub(super) use engine_core::constants::extensions;
pub(super) use engine_core::prelude::*;
pub(super) use engine_core::storage::path_utils::sanitise_name;
pub(super) use std::path::PathBuf;

mod asset_registry_stage_tests;
mod blank_prefab_session_tests;
mod delete_prefab_cmd_tests;
mod movement_tests;
mod prefab_actions_tests;
mod prefab_editor_tests;
mod prefab_room_sync_tests;
mod prefab_save_tests;
mod prefab_transition_tests;
mod selection_tests;

fn make_editor_with_selected_entity(entity: Entity) -> Editor {
    let mut editor = Editor {
        mode: EditorMode::Room(RoomId(1)),
        ..Default::default()
    };
    editor.room_editor.selected_entities.insert(entity);
    editor
}

fn linked_instance_node_count(ecs: &Ecs, prefab_id: PrefabId) -> usize {
    ecs.get_store::<PrefabInstanceNode>()
        .data
        .values()
        .filter(|node| node.prefab_id == prefab_id)
        .count()
}

fn save_test_prefab(test_game: &TestGameFolder, prefab_id: PrefabId, name: &str) -> PrefabAsset {
    let prefab = create_prefab(prefab_id, name.to_string());
    persist_prefab(test_game.name(), &prefab, &AssetRegistry::default(), None)
        .expect("test prefab should persist")
        .0
}

fn saved_prefab_path(prefab: &PrefabAsset) -> PathBuf {
    prefabs_folder().join(format!(
        "{}.{}",
        sanitise_name(&prefab.name),
        extensions::PREFAB
    ))
}

fn blank_prefab_session_editor(test_game: &TestGameFolder) -> Editor {
    set_game_name(test_game.name());
    let game = create_new_game(test_game.name().to_string());
    let prefab_stage = PrefabStage::from_editor_services(&game);

    Editor {
        game,
        mode: EditorMode::Prefab(BLANK_PREFAB_ID),
        return_mode: Some(EditorMode::Room(RoomId(1))),
        prefab_editor: Some(PrefabEditor::new(
            BLANK_PREFAB_ID,
            "Prefab".to_string(),
            StagedPrefabState::Empty,
            PrefabRoomSyncState {
                staged_prefab: StagedPrefabState::Empty,
                linked_instance_snapshots: Vec::new(),
            },
        )),
        prefab_stage: Some(prefab_stage),
        ..Default::default()
    }
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
