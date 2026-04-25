pub(super) use crate::app::{Editor, EditorMode};
pub(super) use crate::commands::editor_command_manager::EditorCommand;
pub(super) use crate::commands::scene::{
    ApplyInstanceToPrefabCmd, DeleteEntityCmd, RevertPrefabInstanceCmd, UnlinkPrefabInstanceCmd,
    UpdateComponentCmd,
};
pub(super) use crate::editor_global::{
    apply_pending_commands, push_command, request_undo, reset_services, set_editor, with_editor,
    EDITOR_SERVICES,
};
pub(super) use crate::prefab::prefab_editor::{
    PrefabEditor, PrefabRoomSyncState, PrefabStage, StagedPrefabState,
};
pub(super) use crate::prefab::tests::install_prefab_save_picker_result;
pub(super) use crate::storage::editor_storage::create_new_game;
pub(super) use engine_core::constants::extensions;
pub(super) use engine_core::prelude::*;
pub(super) use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};

mod apply_instance_to_prefab_cmd_tests;
mod delete_entity_cmd_tests;
mod revert_prefab_instance_cmd_tests;
mod unlink_prefab_instance_cmd_tests;
mod update_component_cmd_tests;

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

fn make_room_editor(test_game: &TestGameFolder) -> (Editor, RoomId) {
    set_game_name(test_game.name());
    let mut game = create_new_game(test_game.name().to_string());
    let cur_world_id = game
        .current_world_id
        .expect("new game should have a current world");
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
    let _picker = install_prefab_save_picker_result(Some(
        prefabs_folder().join(format!("Crate.{}", extensions::PREFAB)),
    ));
    editor.create_prefab_from_selection(root);

    (editor, room_id, PrefabId(1), root)
}
