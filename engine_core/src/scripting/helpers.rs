use crate::ecs::ScriptField;
use crate::scripting::lua_constants::{X, Y, Z};
use bishop::prelude::{Vec2, Vec3};
use mlua::prelude::LuaResult;
use mlua::{Lua, Table, Value};

/// Converts a Rust-style type name into the snake_case Lua API form.
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

/// Reads a public script field value from a Lua value.
pub fn read_script_field(name: &str, value: Value) -> LuaResult<Option<ScriptField>> {
    match value {
        Value::Boolean(b) => Ok(Some(ScriptField::Bool(b))),
        Value::Integer(i) => Ok(Some(ScriptField::Int(i))),
        Value::Number(n) => Ok(Some(ScriptField::Float(n))),
        Value::String(s) => Ok(Some(ScriptField::Text(s.to_str()?.to_string()))),
        Value::Table(table) => read_script_vector_field(name, &table),
        _ => Ok(None),
    }
}

/// Writes a public script field into a Lua table.
pub fn write_script_field(
    lua: &Lua,
    public: &Table,
    name: &str,
    field: &ScriptField,
) -> LuaResult<()> {
    match field {
        ScriptField::Bool(value) => public.set(name, *value)?,
        ScriptField::Int(value) => public.set(name, *value)?,
        ScriptField::Float(value) => public.set(name, *value)?,
        ScriptField::Text(value) => public.set(name, value.clone())?,
        ScriptField::Vec2(value) => public.set(
            name,
            write_named_vec2_table(lua, Vec2::new(value[0], value[1]))?,
        )?,
        ScriptField::Vec3(value) => public.set(
            name,
            write_named_vec3_table(lua, Vec3::new(value[0], value[1], value[2]))?,
        )?,
    }

    Ok(())
}

/// Writes a `Vec2` into a named Lua table.
pub fn write_named_vec2_table(lua: &Lua, value: Vec2) -> LuaResult<Table> {
    let table = lua.create_table()?;
    table.set(X, value.x)?;
    table.set(Y, value.y)?;
    Ok(table)
}

/// Writes a `Vec3` into a named Lua table.
pub fn write_named_vec3_table(lua: &Lua, value: Vec3) -> LuaResult<Table> {
    let table = lua.create_table()?;
    table.set(X, value.x)?;
    table.set(Y, value.y)?;
    table.set(Z, value.z)?;
    Ok(table)
}

/// Reads a named `Vec2` table from Lua for the provided validation context.
pub fn parse_named_vec2(table: &Table, context: &str) -> LuaResult<Vec2> {
    reject_indexed_vector_keys(table, context)?;
    let x = table
        .get::<Option<f32>>(X)?
        .ok_or_else(|| named_vector_error(context))?;
    let y = table
        .get::<Option<f32>>(Y)?
        .ok_or_else(|| named_vector_error(context))?;

    Ok(Vec2::new(x, y))
}

/// Reads a named `Vec3` table from Lua.
pub fn read_named_vec3_table(table: &Table, field_name: &str) -> LuaResult<Vec3> {
    let context = format!("Script field '{field_name}'");
    reject_indexed_vector_keys(table, &context)?;
    let x = table
        .get::<Option<f32>>(X)?
        .ok_or_else(|| named_vector_error(&context))?;
    let y = table
        .get::<Option<f32>>(Y)?
        .ok_or_else(|| named_vector_error(&context))?;
    let z = table
        .get::<Option<f32>>(Z)?
        .ok_or_else(|| named_vector_error(&context))?;

    Ok(Vec3::new(x, y, z))
}

fn read_script_vector_field(name: &str, table: &Table) -> LuaResult<Option<ScriptField>> {
    let context = format!("Script field '{name}'");
    reject_indexed_vector_keys(table, &context)?;
    let x = table.get::<Option<f32>>(X)?;
    let y = table.get::<Option<f32>>(Y)?;
    let z = table.get::<Option<f32>>(Z)?;

    match (x, y, z) {
        (Some(_), Some(_), Some(_)) => {
            let vec = read_named_vec3_table(table, name)?;
            Ok(Some(ScriptField::Vec3([vec.x, vec.y, vec.z])))
        }
        (Some(_), Some(_), None) => {
            let vec = parse_named_vec2(table, &format!("Script field '{name}'"))?;
            Ok(Some(ScriptField::Vec2([vec.x, vec.y])))
        }
        (None, None, None) => Ok(None),
        _ => Err(named_vector_error(&context)),
    }
}

fn reject_indexed_vector_keys(table: &Table, context: &str) -> LuaResult<()> {
    if (1..=3).any(|index| {
        matches!(
            table.get::<Value>(index).ok(),
            Some(Value::Number(_) | Value::Integer(_))
        )
    }) {
        return Err(named_vector_error(context));
    }
    Ok(())
}

fn named_vector_error(context: &str) -> mlua::Error {
    mlua::Error::RuntimeError(format!(
        "{context} must use named vector table {{ {X} = number, {Y} = number }}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::lua_constants::POSITION;

    fn assert_named_vec2_table(table: &Table, expected_x: f32, expected_y: f32) {
        assert_eq!(table.get::<f32>(X).unwrap(), expected_x);
        assert_eq!(table.get::<f32>(Y).unwrap(), expected_y);
        assert!(matches!(table.get::<Value>(1).unwrap(), Value::Nil));
        assert!(matches!(table.get::<Value>(2).unwrap(), Value::Nil));
    }

    fn assert_named_vec3_table(table: &Table, expected_x: f32, expected_y: f32, expected_z: f32) {
        assert_eq!(table.get::<f32>(X).unwrap(), expected_x);
        assert_eq!(table.get::<f32>(Y).unwrap(), expected_y);
        assert_eq!(table.get::<f32>(Z).unwrap(), expected_z);
        assert!(matches!(table.get::<Value>(1).unwrap(), Value::Nil));
        assert!(matches!(table.get::<Value>(2).unwrap(), Value::Nil));
        assert!(matches!(table.get::<Value>(3).unwrap(), Value::Nil));
    }

    #[test]
    fn to_snake_case_inserts_underscores_before_uppercase_letters() {
        assert_eq!(to_snake_case("PrefabInstanceRoot"), "prefab_instance_root");
    }

    #[test]
    fn write_script_field_writes_named_vec_tables() {
        let lua = Lua::new();
        let public = lua.create_table().unwrap();

        write_script_field(&lua, &public, POSITION, &ScriptField::Vec2([12.5, -3.0])).unwrap();
        write_script_field(
            &lua,
            &public,
            "color",
            &ScriptField::Vec3([0.25, 0.5, 0.75]),
        )
        .unwrap();

        let position: Table = public.get(POSITION).unwrap();
        let color: Table = public.get("color").unwrap();
        assert_named_vec2_table(&position, 12.5, -3.0);
        assert_named_vec3_table(&color, 0.25, 0.5, 0.75);
    }

    #[test]
    fn read_script_field_reads_named_vec_tables() {
        let lua = Lua::new();
        let position = lua.create_table().unwrap();
        let color = lua.create_table().unwrap();

        position.set(X, 12.5).unwrap();
        position.set(Y, -3.0).unwrap();
        color.set(X, 0.25).unwrap();
        color.set(Y, 0.5).unwrap();
        color.set(Z, 0.75).unwrap();

        let position_field = read_script_field(POSITION, Value::Table(position)).unwrap();
        let color_field = read_script_field("color", Value::Table(color)).unwrap();

        assert!(matches!(
            position_field,
            Some(ScriptField::Vec2(v)) if v == [12.5, -3.0]
        ));
        assert!(matches!(
            color_field,
            Some(ScriptField::Vec3(v)) if v == [0.25, 0.5, 0.75]
        ));
    }

    #[test]
    fn read_script_field_rejects_indexed_vec_tables() {
        let lua = Lua::new();
        let position = lua.create_table().unwrap();
        position.set(1, 12.5).unwrap();
        position.set(2, -3.0).unwrap();

        let error = read_script_field(POSITION, Value::Table(position)).unwrap_err();

        assert!(error.to_string().contains(POSITION));
    }

    #[test]
    fn read_named_vec2_reads_named_fields() {
        let lua = Lua::new();
        let position = lua.create_table().unwrap();
        position.set(X, 12.5).unwrap();
        position.set(Y, -3.0).unwrap();

        let position = parse_named_vec2(&position, "position").unwrap();

        assert_eq!(position, Vec2::new(12.5, -3.0));
    }

    #[test]
    fn read_named_vec2_rejects_indexed_tables() {
        let lua = Lua::new();
        let position = lua.create_table().unwrap();
        position.set(1, 12.5).unwrap();
        position.set(2, -3.0).unwrap();

        assert!(parse_named_vec2(&position, "Entity:teleport position").is_err());
    }
}
