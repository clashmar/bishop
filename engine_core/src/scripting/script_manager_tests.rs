use super::*;
use crate::assets::asset_registry::AssetKey;
use crate::constants::paths;
use crate::engine_global::set_game_name;
use crate::scripting::lua_constants::{lua_dirs, lua_files, lua_globals};
use crate::scripting::lua_project::engine_require_path;
use crate::storage::test_utils::{TestGameFolder, game_fs_test_lock};

#[test]
fn get_or_load_registers_new_script_path_in_asset_registry() {
    let mut registry = AssetRegistry::default();
    let mut script_manager = ScriptManager::default();
    let path = PathBuf::from("player.lua");

    let result = script_manager.get_or_load(&mut registry, &path);

    assert_eq!(result, Some(ScriptId(1)));
    assert_eq!(
        registry.key_for_path(PathBuf::from(paths::SCRIPTS_FOLDER).join(&path)),
        Some(AssetKey::Script(ScriptId(1)))
    );
    assert_eq!(script_manager.path_to_script_id.get(&path), Some(&ScriptId(1)));
}

#[test]
fn pending_init_count_returns_pending_queue_length() {
    let mut manager = ScriptManager::default();
    manager.pending_inits.push((Entity(1), ScriptId(9)));
    manager.pending_inits.push((Entity(2), ScriptId(9)));

    assert_eq!(manager.pending_init_count(), 2);
}

#[test]
fn evict_script_removes_cached_definition_when_no_instances_reference_it() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_evict_script");
    set_game_name(folder.name());

    let lua = Lua::new();
    let mut registry = AssetRegistry::default();
    let mut manager = ScriptManager::default();

    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join("enemy.lua"),
        "return { update = function() end }",
    )
    .unwrap();

    let script_id = manager.init_script(&mut registry, "enemy.lua").unwrap();
    let _ = manager.load_script_table(&lua, script_id).unwrap();
    assert_eq!(manager.loaded_script_count(), 1);

    manager.evict_script(script_id);
    assert_eq!(manager.loaded_script_count(), 0);
}

#[test]
fn load_globals_prelude_bootstraps_globals_for_editor_style_script_loads() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_editor_globals");
    set_game_name(folder.name());

    let engine_data_dir = scripts_folder().join(lua_dirs::ENGINE).join(lua_dirs::DATA);
    fs::create_dir_all(&engine_data_dir).unwrap();
    fs::write(
        scripts_folder().join(lua_dirs::ENGINE).join(lua_files::GLOBALS),
        format!(
            "{} = require(\"{}\")\n",
            lua_globals::DIRECTION,
            engine_require_path(lua_files::DIRECTION)
        ),
    )
    .unwrap();
    fs::write(
        engine_data_dir.join(lua_files::DIRECTION),
        "return { Right = \"right\" }\n",
    )
    .unwrap();
    fs::write(
        scripts_folder().join("probe.lua"),
        format!(
            "return {{ public = {{ facing = {}.Right }} }}\n",
            lua_globals::DIRECTION
        ),
    )
    .unwrap();

    let mut registry = AssetRegistry::default();
    let mut script_manager = ScriptManager::default();
    let script_id = script_manager
        .get_or_load(&mut registry, PathBuf::from("probe.lua"))
        .unwrap();
    let lua = Lua::new();

    ScriptManager::load_to_package(&lua);
    ScriptManager::load_globals_prelude(&lua).unwrap();

    let table = script_manager.get_table_from_id(&lua, script_id).unwrap();
    let public: Table = table.get(lua_fields::PUBLIC).unwrap();

    assert!(public.get::<String>("facing").is_ok());
}

#[test]
fn unload_removes_only_specific_instance() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_unload_one");
    set_game_name(folder.name());

    let lua = Lua::new();
    let mut registry = AssetRegistry::default();
    let mut manager = ScriptManager::default();

    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join("a.lua"),
        "return { update = function() end }",
    )
    .unwrap();
    fs::write(
        scripts_folder().join("b.lua"),
        "return { update = function() end }",
    )
    .unwrap();

    let a_id = manager.init_script(&mut registry, "a.lua").unwrap();
    let b_id = manager.init_script(&mut registry, "b.lua").unwrap();
    let entity = Entity(1);

    let _ = manager.get_or_create_instance(&lua, entity, a_id).unwrap();
    let _ = manager.get_or_create_instance(&lua, entity, b_id).unwrap();
    assert_eq!(manager.instance_count(), 2);

    manager.unload(entity, a_id);

    assert_eq!(manager.instance_count(), 1);
    assert!(manager.instances.contains_key(&(entity, b_id)));
    assert!(!manager.instances.contains_key(&(entity, a_id)));
}

#[test]
fn unload_all_for_entity_removes_all_instances() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_unload_all");
    set_game_name(folder.name());

    let lua = Lua::new();
    let mut registry = AssetRegistry::default();
    let mut manager = ScriptManager::default();

    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join("a.lua"),
        "return { update = function() end }",
    )
    .unwrap();
    fs::write(
        scripts_folder().join("b.lua"),
        "return { update = function() end }",
    )
    .unwrap();

    let a_id = manager.init_script(&mut registry, "a.lua").unwrap();
    let b_id = manager.init_script(&mut registry, "b.lua").unwrap();
    let entity = Entity(1);

    let _ = manager.get_or_create_instance(&lua, entity, a_id).unwrap();
    let _ = manager.get_or_create_instance(&lua, entity, b_id).unwrap();
    assert_eq!(manager.instance_count(), 2);

    manager.unload_all_for_entity(entity);

    assert_eq!(manager.instance_count(), 0);
}

#[test]
fn unload_never_loaded_script_leaves_cache_empty() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_unload_never_loaded");
    set_game_name(folder.name());

    let mut registry = AssetRegistry::default();
    let mut manager = ScriptManager::default();

    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join("a.lua"),
        "return { update = function() end }",
    )
    .unwrap();

    let a_id = manager.init_script(&mut registry, "a.lua").unwrap();

    manager.unload(Entity(1), a_id);

    assert_eq!(manager.loaded_script_count(), 0);
    assert_eq!(manager.ref_count(&a_id), 0);
}

#[test]
fn evict_script_refuses_when_coordinator_ref_count_positive() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_hydratable");
    set_game_name(folder.name());
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(scripts_folder().join("test.lua"), "return {}").unwrap();

    let lua = Lua::new();
    let mut registry = AssetRegistry::default();
    let mut sm = ScriptManager::default();
    let id = sm.init_script(&mut registry, "test.lua").unwrap();
    sm.load_script_table(&lua, id).unwrap();

    sm.increment_ref(id);
    let result = sm.evict(&id);
    assert_eq!(
        result,
        Err(crate::hydration::EvictError::StillReferenced { count: 2 })
    );
    assert_eq!(sm.loaded_script_count(), 1);
}

#[test]
fn evict_script_refuses_when_instances_exist_even_if_ref_count_zero() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_instances");
    set_game_name(folder.name());
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(scripts_folder().join("test.lua"), "return {}").unwrap();

    let lua = Lua::new();
    let mut registry = AssetRegistry::default();
    let mut sm = ScriptManager::default();
    let id = sm.init_script(&mut registry, "test.lua").unwrap();
    sm.load_script_table(&lua, id).unwrap();

    let entity = Entity(1);
    sm.get_or_create_instance(&lua, entity, id).unwrap();

    sm.decrement_ref(id);
    let result = sm.evict(&id);
    assert_eq!(result, Err(crate::hydration::EvictError::HasLiveConsumers));
}

#[test]
fn interact_fns_populated_on_load() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_interact_load");
    set_game_name(folder.name());
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join("npc.lua"),
        "return { interact = function() end }",
    )
    .unwrap();

    let lua = Lua::new();
    let mut registry = AssetRegistry::default();
    let mut manager = ScriptManager::default();
    let id = manager.init_script(&mut registry, "npc.lua").unwrap();
    manager.load_script_table(&lua, id).unwrap();

    assert!(manager.interact_fns.contains_key(&id), "interact_fns should contain script with interact method");
}

#[test]
fn interact_fns_empty_when_no_interact() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_no_interact");
    set_game_name(folder.name());
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join("prop.lua"),
        "return { update = function() end }",
    )
    .unwrap();

    let lua = Lua::new();
    let mut registry = AssetRegistry::default();
    let mut manager = ScriptManager::default();
    let id = manager.init_script(&mut registry, "prop.lua").unwrap();
    manager.load_script_table(&lua, id).unwrap();

    assert!(!manager.interact_fns.contains_key(&id), "interact_fns should not contain script without interact method");
}

#[test]
fn script_has_interact_returns_correct_bool() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_has_interact");
    set_game_name(folder.name());
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join("npc.lua"),
        "return { interact = function() end }",
    )
    .unwrap();
    fs::write(
        scripts_folder().join("prop.lua"),
        "return { update = function() end }",
    )
    .unwrap();

    let lua = Lua::new();
    let mut registry = AssetRegistry::default();
    let mut manager = ScriptManager::default();
    let npc_id = manager.init_script(&mut registry, "npc.lua").unwrap();
    let prop_id = manager.init_script(&mut registry, "prop.lua").unwrap();
    manager.load_script_table(&lua, npc_id).unwrap();
    manager.load_script_table(&lua, prop_id).unwrap();

    assert!(manager.script_has_interact(npc_id), "npc script should report having interact");
    assert!(!manager.script_has_interact(prop_id), "prop script should not report having interact");
    assert!(!manager.script_has_interact(ScriptId(999)), "non-existent script id should not report having interact");
}

#[test]
fn interact_fns_cleaned_on_evict() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("script_manager_interact_evict");
    set_game_name(folder.name());
    fs::create_dir_all(scripts_folder()).unwrap();
    fs::write(
        scripts_folder().join("npc.lua"),
        "return { interact = function() end }",
    )
    .unwrap();

    let lua = Lua::new();
    let mut registry = AssetRegistry::default();
    let mut manager = ScriptManager::default();
    let id = manager.init_script(&mut registry, "npc.lua").unwrap();
    manager.load_script_table(&lua, id).unwrap();

    assert!(manager.interact_fns.contains_key(&id));

    manager.evict_script(id);

    assert!(!manager.interact_fns.contains_key(&id), "interact_fns entry should be removed after evict");
}