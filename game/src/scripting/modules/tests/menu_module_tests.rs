use crate::game_global::drain_commands;
use crate::scripting::modules::menu_module::MenuModule;
use engine_core::scripting::lua_constants::lua_menu;
use engine_core::scripting::LuaModule;
use engine_core::scripting::modules::lua_module::{LuaApi, LuaApiWriter};
use mlua::Lua;

fn setup_menu_lua() -> Lua {
    let lua = Lua::new();
    lua.globals()
        .set("engine", lua.create_table().unwrap())
        .unwrap();
    MenuModule.register(&lua).unwrap();
    lua
}

#[test]
fn set_enabled_enqueues_one_command() {
    let lua = setup_menu_lua();
    lua.load("engine.menu.set_enabled('test', 'BtnA', false)")
        .exec()
        .unwrap();
    assert_eq!(drain_commands().count(), 1);
}

#[test]
fn set_visible_enqueues_one_command() {
    let lua = setup_menu_lua();
    lua.load("engine.menu.set_visible('test', 'BtnB', true)")
        .exec()
        .unwrap();
    assert_eq!(drain_commands().count(), 1);
}

#[test]
fn menus_table_is_accepted_for_open_and_mutation_calls() {
    let lua = setup_menu_lua();
    lua.load(
        r#"
        local menu = { Title = { Id = "title", LoadGame = "LoadGame" } }
        engine.menu.open(menu.Title)
        engine.menu.set_enabled(menu.Title, menu.Title.LoadGame, false)
        "#,
    )
    .exec()
    .unwrap();

    assert_eq!(drain_commands().count(), 2);
}

#[test]
fn emit_api_documents_menus_type() {
    let mut out = LuaApiWriter::default();
    MenuModule.emit_api(&mut out);

    let ref_param = format!("---@param menu string|{}", lua_menu::MENUS_CLASS);
    assert!(out.buf.contains(&ref_param));
    assert!(out.buf.contains("function engine.menu.set_enabled(menu, element_name, enabled) end"));
    assert!(out.buf.contains("function engine.menu.set_visible(menu, element_name, visible) end"));
}
