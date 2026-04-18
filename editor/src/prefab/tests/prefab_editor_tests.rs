use super::*;
use std::path::Path;

#[test]
fn prefab_stage_uses_project_sprite_paths_without_room_state() {
    const CAT_SPRITE_PATH: &str = "sprites/cat.png";

    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_stage_game");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    game.asset_registry
        .register_asset_relative_path(SpriteId(7), CAT_SPRITE_PATH)
        .expect("sprite path should register");
    save_game(&game).unwrap();

    let mut stage = PrefabStage::new(test_game.name());
    let prefab_ctx = stage.ctx_mut();

    assert_eq!(
        prefab_ctx.sprite_manager.path_for_id(SpriteId(7)),
        Some(Path::new(CAT_SPRITE_PATH))
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
