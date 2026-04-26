use crate::app::{Editor, EditorMode};
use crate::editor_global::{reset_services, set_editor, EDITOR_SERVICES};
use crate::storage::editor_storage::create_new_game;
use engine_core::constants::extensions;
use engine_core::prelude::*;
pub use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

static TEST_PREFAB_SAVE_PICKER_RESULT: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

pub(crate) struct TestPrefabSavePickerGuard;

impl Drop for TestPrefabSavePickerGuard {
    fn drop(&mut self) {
        *TEST_PREFAB_SAVE_PICKER_RESULT
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = None;
    }
}

pub(crate) fn install_prefab_save_picker_result(
    result: Option<PathBuf>,
) -> TestPrefabSavePickerGuard {
    *TEST_PREFAB_SAVE_PICKER_RESULT
        .lock()
        .unwrap_or_else(|poison| poison.into_inner()) = Some(result);
    TestPrefabSavePickerGuard
}

pub(crate) fn take_test_prefab_save_picker_result() -> Option<Option<PathBuf>> {
    TEST_PREFAB_SAVE_PICKER_RESULT
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take()
}

pub struct EditorServicesGuard;

impl EditorServicesGuard {
    pub fn install(editor: Editor) -> Self {
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

/// Holds the game-fs lock, test game folder, and editor services guard together
/// so that drop order guarantees the lock outlives the test game folder teardown.
pub struct TestEditorContext {
    pub _lock: MutexGuard<'static, ()>,
    pub _test_game: TestGameFolder,
    pub _guard: EditorServicesGuard,
}

/// Creates a fresh editor with an isolated test game folder and acquires the
/// global game-fs lock. Returns a `TestEditorContext` whose drop order ensures
/// the lock outlives the `TestGameFolder` cleanup.
pub fn setup_editor(test_prefix: &str) -> TestEditorContext {
    let lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new(test_prefix);
    set_game_name(test_game.name());
    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let guard = EditorServicesGuard::install(editor);
    TestEditorContext {
        _lock: lock,
        _test_game: test_game,
        _guard: guard,
    }
}

pub fn make_room_editor(test_game: &TestGameFolder) -> (Editor, RoomId) {
    set_game_name(test_game.name());
    let mut game = create_new_game(test_game.name().to_string());
    let cur_world_id = game
        .current_world_id
        .expect("new game should have a current world");
    let room_id = game
        .current_world()
        .starting_room_id
        .expect("new game should have a starting room");
    game.get_world_mut(cur_world_id)
        .expect("new game should have world")
        .current_room_id = Some(room_id);
    let editor = Editor {
        game,
        mode: EditorMode::Room(room_id),
        cur_world_id: Some(cur_world_id),
        cur_room_id: Some(room_id),
        ..Default::default()
    };

    (editor, room_id)
}

pub fn make_prefab_session_editor(
    test_game: &TestGameFolder,
) -> (Editor, RoomId, PrefabId, Entity) {
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

pub fn linked_root_entities(ecs: &Ecs, prefab_id: PrefabId) -> Vec<Entity> {
    ecs.get_store::<PrefabInstanceRoot>()
        .data
        .iter()
        .filter_map(|(&entity, root)| (root.prefab_id == prefab_id).then_some(entity))
        .collect()
}
