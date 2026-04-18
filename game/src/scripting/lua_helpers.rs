// game/src/scripting/lua_helpers.rs
use bishop::prelude::Vec2;
use engine_core::scripting::lua_constants::{X, Y};
use mlua::prelude::LuaResult;
use mlua::{Table, Value};

pub fn to_snake_case(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn parse_named_vec2(method_name: &str, table: &Table) -> LuaResult<Vec2> {
    let expected = format!("{method_name} requires {{ {X} = number, {Y} = number }}");
    let invalid_vec2 = || mlua::Error::RuntimeError(expected.clone());

    let x = table.get::<f32>(X).map_err(|_| invalid_vec2())?;
    let y = table.get::<f32>(Y).map_err(|_| invalid_vec2())?;

    if (1..=3).any(|index| {
        matches!(
            table.get::<Value>(index).ok(),
            Some(Value::Number(_) | Value::Integer(_))
        )
    }) {
        return Err(invalid_vec2());
    }

    Ok(Vec2::new(x, y))
}
