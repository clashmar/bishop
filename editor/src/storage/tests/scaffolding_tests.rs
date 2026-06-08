use super::*;

#[test]
fn create_game_folders_scaffolds_lua_project_files() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("lua_scaffold_files");
    set_game_name(test_game.name());

    create_game_folders(test_game.name());

    let game_root = game_folder(test_game.name());
    let engine_folder = scripts_folder().join(lua_dirs::ENGINE);

    assert!(
        editor_metadata_folder(test_game.name()).exists(),
        "game root should include _editor metadata folder"
    );
    assert!(
        game_root.join(lua_files::LUARC).exists(),
        "game root should include .luarc.json"
    );
    assert!(
        game_root.join(lua_files::LUACHECK).exists(),
        "game root should include .luacheckrc"
    );
    assert!(
        game_root.join(lua_files::STYLUA).exists(),
        "game root should include stylua.toml"
    );
    assert!(
        engine_folder.join(lua_files::GLOBALS).exists(),
        "_engine/globals.lua should be scaffolded"
    );
}

#[test]
fn create_new_game_seeds_generated_lua_tables_for_globals_prelude() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("lua_generated_tables");
    set_game_name(test_game.name());

    let _game = create_new_game(test_game.name().to_string());

    let engine_folder = scripts_folder().join(lua_dirs::ENGINE);
    for filename in [
        lua_files::ANIMATIONS,
        lua_files::PREFABS,
        lua_files::SOUNDS,
        lua_files::MENUS,
    ] {
        assert!(
            engine_folder
                .join(engine_core::scripting::lua_project::engine_relative_path(filename))
                .exists(),
            "{filename} should exist for globals prelude consumers"
        );
    }
}

#[test]
fn create_new_game_creates_bishop_theme() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("bishop_theme_created");
    set_game_name(test_game.name());

    let _game = create_new_game(test_game.name().to_string());

    let theme_path = themes_folder().join(lua_files::BISHOP_THEME);
    assert!(theme_path.exists(), "bishop_theme.lua should exist");
    let content = fs::read_to_string(theme_path).unwrap();
    assert!(
        content.contains("local t = engine.theme.new()"),
        "theme should contain expected header"
    );
}
