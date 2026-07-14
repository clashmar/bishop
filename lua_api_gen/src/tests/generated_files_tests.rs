use super::collect_generated_files;
use engine_core::scripting::lua_constants::lua_files;

#[test]
fn collect_generated_files_when_called_sorts_engine_lua_contributors_by_module_name() {
    let generated = collect_generated_files();
    let engine_lua = generated
        .get(lua_files::ENGINE)
        .expect("engine.lua should be generated");

    let engine_index = engine_lua
        .find("engine.asset = {}")
        .expect("engine module marker should exist");
    let input_index = engine_lua
        .find("function engine.input.is_down(input) end")
        .expect("input module marker should exist");
    let logging_index = engine_lua
        .find("function engine.log.info(msg) end")
        .expect("logging module marker should exist");
    let prefab_index = engine_lua
        .find("engine.prefab = {}")
        .expect("prefab module marker should exist");

    assert!(engine_index < input_index);
    assert!(input_index < logging_index);
    assert!(logging_index < prefab_index);
}
