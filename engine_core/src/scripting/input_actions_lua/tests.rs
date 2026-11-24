use super::{
    create_input_actions_lua_table, generate_input_actions_lua, input_action_lua_entries,
};
use crate::input::{InputActionId, InputActionMap};
use crate::scripting::lua_constants::{lua_globals, lua_ownership};
use mlua::Lua;
use std::collections::HashMap;

#[test]
fn generated_action_lua_uses_sanitized_values_from_the_authored_map() {
    let action_map = InputActionMap {
        actions: HashMap::from([
            (InputActionId::new("move-right"), Vec::new()),
            (InputActionId::new("jump"), Vec::new()),
        ]),
    };

    let entries = input_action_lua_entries(&action_map);
    let lua = generate_input_actions_lua(&action_map);

    assert_eq!(
        entries,
        vec![
            ("Jump".to_string(), "jump".to_string()),
            ("MoveRight".to_string(), "move-right".to_string()),
        ]
    );
    assert!(lua.contains(lua_ownership::LUA_OWNER_GAME_GENERATED));
    assert!(lua.contains(&format!("---@enum {}", lua_globals::INPUT_ACTION)));
    assert!(lua.contains(&format!("{} = {{", lua_globals::INPUT_ACTION)));
    assert!(lua.contains("Jump = \"jump\""));
    assert!(lua.contains("MoveRight = \"move-right\""));
}

#[test]
fn generated_action_lua_disambiguates_sanitized_name_collisions() {
    let action_map = InputActionMap {
        actions: HashMap::from([
            (InputActionId::new("move-right"), Vec::new()),
            (InputActionId::new("move right"), Vec::new()),
        ]),
    };

    let entries = input_action_lua_entries(&action_map);

    assert_eq!(entries[0].0, "MoveRight");
    assert_eq!(entries[1].0, "MoveRight_2");
}

#[test]
fn generated_action_lua_escapes_identifiers_as_lua_strings() -> mlua::Result<()> {
    let action_map = InputActionMap {
        actions: HashMap::from([(InputActionId::new("jump \"high\""), Vec::new())]),
    };
    let generated_lua = generate_input_actions_lua(&action_map);
    let lua = Lua::new();

    lua.load(&generated_lua).exec()?;
    let actions = lua.globals().get::<mlua::Table>(lua_globals::INPUT_ACTION)?;
    assert_eq!(actions.get::<String>("JumpHigh")?, "jump \"high\"");
    Ok(())
}

#[test]
fn runtime_action_table_values_match_authored_identifiers() -> mlua::Result<()> {
    let action_map = InputActionMap {
        actions: HashMap::from([(InputActionId::new("jump"), Vec::new())]),
    };
    let lua = Lua::new();
    let table = create_input_actions_lua_table(&lua, &action_map)?;

    assert_eq!(table.get::<String>("Jump")?, "jump");
    Ok(())
}
