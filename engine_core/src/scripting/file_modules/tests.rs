use std::fs;
use std::sync::MutexGuard;

use mlua::{Lua, Table};

use super::{INIT_FILE, install};
use crate::engine_global::set_game_name;
use crate::scripting::lua_constants::{lua_dirs, lua_files};
use crate::scripting::lua_project::engine_require_path;
use crate::storage::path_utils::{scripts_folder, themes_folder};
use crate::storage::test_utils::{TestGameFolder, game_fs_test_lock};

struct TestScriptGame {
    _folder: TestGameFolder,
    _lock: MutexGuard<'static, ()>,
}

impl TestScriptGame {
    fn new(name: &str) -> Self {
        let lock = game_fs_test_lock().lock().unwrap();
        let folder = TestGameFolder::new(name);
        set_game_name(folder.name());
        Self {
            _folder: folder,
            _lock: lock,
        }
    }
}

#[test]
fn file_require_when_dotted_module_exists_loads_from_scripts_folder() {
    let _game = TestScriptGame::new("file_require_dotted_module");

    const TEST_FIELD: &str = "ready";
    let module_path = scripts_folder()
        .join(lua_dirs::ENGINE)
        .join(lua_dirs::DATA)
        .join(lua_files::DIRECTION);
    fs::create_dir_all(module_path.parent().unwrap()).unwrap();
    fs::write(&module_path, format!("return {{ {TEST_FIELD} = true }}")).unwrap();

    let lua = Lua::new();
    install(&lua).unwrap();
    let module: Table = lua
        .load(format!(
            "return require(\"{}\")",
            engine_require_path(lua_files::DIRECTION)
        ))
        .eval()
        .unwrap();
    assert!(module.get::<bool>(TEST_FIELD).unwrap());
}

#[test]
fn file_require_when_module_is_loaded_twice_evaluates_it_once() {
    let _game = TestScriptGame::new("file_require_cache");

    const MODULE_NAME: &str = "counter";
    const RUN_COUNT: &str = "run_count";
    let module_path = scripts_folder().join(format!("{MODULE_NAME}.lua"));
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        &module_path,
        format!("{RUN_COUNT} = ({RUN_COUNT} or 0) + 1\nreturn {{}}"),
    )
    .unwrap();

    let lua = Lua::new();
    install(&lua).unwrap();
    lua.load(format!(
        "require(\"{MODULE_NAME}\"); require(\"{MODULE_NAME}\")"
    ))
    .exec()
    .unwrap();
    assert_eq!(lua.globals().get::<u32>(RUN_COUNT).unwrap(), 1);
}

#[test]
fn file_require_when_engine_module_is_registered_keeps_luau_resolution() {
    const ENGINE_MODULE: &str = "@engine";
    let lua = Lua::new();
    let engine = lua.create_table().unwrap();
    lua.register_module(ENGINE_MODULE, &engine).unwrap();

    install(&lua).unwrap();
    let loaded: Table = lua
        .load(format!("return require(\"{ENGINE_MODULE}\")"))
        .eval()
        .unwrap();
    assert_eq!(loaded, engine);
}

#[test]
fn file_require_when_directory_has_init_file_loads_its_module() {
    let _game = TestScriptGame::new("file_require_init_module");

    const MODULE_NAME: &str = "nested";
    const TEST_FIELD: &str = "loaded";
    let module_path = scripts_folder().join(MODULE_NAME).join(INIT_FILE);
    fs::create_dir_all(module_path.parent().unwrap()).unwrap();
    fs::write(&module_path, format!("return {{ {TEST_FIELD} = true }}")).unwrap();

    let lua = Lua::new();
    install(&lua).unwrap();
    let module: Table = lua
        .load(format!("return require(\"{MODULE_NAME}\")"))
        .eval()
        .unwrap();
    assert!(module.get::<bool>(TEST_FIELD).unwrap());
}

#[test]
fn file_require_when_theme_module_exists_loads_from_themes_folder() {
    let _game = TestScriptGame::new("file_require_theme_module");

    const MODULE_NAME: &str = "theme_probe";
    const TEST_FIELD: &str = "ready";
    let module_path = themes_folder().join(format!("{MODULE_NAME}.lua"));
    fs::create_dir_all(themes_folder()).unwrap();
    fs::write(&module_path, format!("return {{ {TEST_FIELD} = true }}")).unwrap();

    let lua = Lua::new();
    install(&lua).unwrap();
    let module: Table = lua
        .load(format!("return require(\"{MODULE_NAME}\")"))
        .eval()
        .unwrap();
    assert!(module.get::<bool>(TEST_FIELD).unwrap());
}

#[test]
fn file_require_when_installed_twice_preserves_loaded_module_cache() {
    let _game = TestScriptGame::new("file_require_reinstall");

    const MODULE_NAME: &str = "counter";
    const RUN_COUNT: &str = "run_count";
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join(format!("{MODULE_NAME}.lua")),
        format!("{RUN_COUNT} = ({RUN_COUNT} or 0) + 1\nreturn {{}}"),
    )
    .unwrap();

    let lua = Lua::new();
    install(&lua).unwrap();
    lua.load(format!("require(\"{MODULE_NAME}\")")).exec().unwrap();
    install(&lua).unwrap();
    lua.load(format!("require(\"{MODULE_NAME}\")")).exec().unwrap();
    assert_eq!(lua.globals().get::<u32>(RUN_COUNT).unwrap(), 1);
}

#[test]
fn file_require_when_module_returns_nil_caches_true_result() {
    let _game = TestScriptGame::new("file_require_nil_module");

    const MODULE_NAME: &str = "empty";
    const RUN_COUNT: &str = "run_count";
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join(format!("{MODULE_NAME}.lua")),
        format!("{RUN_COUNT} = ({RUN_COUNT} or 0) + 1"),
    )
    .unwrap();

    let lua = Lua::new();
    install(&lua).unwrap();
    let (first, second): (bool, bool) = lua
        .load(format!(
            "return require(\"{MODULE_NAME}\"), require(\"{MODULE_NAME}\")"
        ))
        .eval()
        .unwrap();
    assert!(first && second);
    assert_eq!(lua.globals().get::<u32>(RUN_COUNT).unwrap(), 1);
}

#[test]
fn file_require_when_module_requires_itself_returns_cached_placeholder() {
    let _game = TestScriptGame::new("file_require_cycle");

    const MODULE_NAME: &str = "self_ref";
    const RUN_COUNT: &str = "run_count";
    const TEST_FIELD: &str = "recursive";
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join(format!("{MODULE_NAME}.lua")),
        format!(
            "{RUN_COUNT} = ({RUN_COUNT} or 0) + 1\nif {RUN_COUNT} > 3 then error('require cycle') end\nreturn {{ {TEST_FIELD} = require('{MODULE_NAME}') == true }}"
        ),
    )
    .unwrap();

    let lua = Lua::new();
    install(&lua).unwrap();
    let module: Table = lua
        .load(format!("return require(\"{MODULE_NAME}\")"))
        .eval()
        .unwrap();
    assert!(module.get::<bool>(TEST_FIELD).unwrap());
    assert_eq!(lua.globals().get::<u32>(RUN_COUNT).unwrap(), 1);
}

#[test]
fn file_require_when_module_is_missing_reports_the_searched_path() {
    let _game = TestScriptGame::new("file_require_missing_module");

    const MODULE_NAME: &str = "missing";
    let lua = Lua::new();
    install(&lua).unwrap();
    let error = lua
        .load(format!("return require(\"{MODULE_NAME}\")"))
        .eval::<mlua::Value>()
        .unwrap_err();
    let searched_path = themes_folder().join(format!("{MODULE_NAME}.lua"));
    assert!(error.to_string().contains(&searched_path.display().to_string()));
}
