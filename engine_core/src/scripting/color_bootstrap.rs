use crate::scripting::lua_constants::lua_color;
use mlua::prelude::LuaResult;
use mlua::Lua;

pub(crate) fn register_color_helpers(lua: &Lua) -> LuaResult<()> {
    let color_tbl = lua.create_table()?;

    let hex_fn = lua.create_function(|lua, (hex_str, alpha): (String, Option<f64>)| {
        let hex = hex_str.trim_start_matches('#');
        let (r, g, b, a) = match hex.len() {
            3 => {
                let r = hex_to_f32(hex, 0, 1)?;
                let g = hex_to_f32(hex, 1, 1)?;
                let b = hex_to_f32(hex, 2, 1)?;
                (r, g, b, alpha.unwrap_or(1.0))
            }
            4 => {
                let r = hex_to_f32(hex, 0, 1)?;
                let g = hex_to_f32(hex, 1, 1)?;
                let b = hex_to_f32(hex, 2, 1)?;
                let a = hex_to_f32(hex, 3, 1)?;
                (r, g, b, a as f64)
            }
            6 => {
                let r = hex_to_f32(hex, 0, 2)?;
                let g = hex_to_f32(hex, 2, 2)?;
                let b = hex_to_f32(hex, 4, 2)?;
                (r, g, b, alpha.unwrap_or(1.0))
            }
            8 => {
                let r = hex_to_f32(hex, 0, 2)?;
                let g = hex_to_f32(hex, 2, 2)?;
                let b = hex_to_f32(hex, 4, 2)?;
                let a = hex_to_f32(hex, 6, 2)?;
                (r, g, b, a as f64)
            }
            _ => return Err(mlua::Error::RuntimeError(format!("invalid hex: {hex_str}"))),
        };
        let ct = lua.create_table()?;
        ct.set(lua_color::R, r)?;
        ct.set(lua_color::G, g)?;
        ct.set(lua_color::B, b)?;
        ct.set(lua_color::A, a)?;
        Ok(ct)
    })?;
    color_tbl.set(lua_color::FROM_HEX, hex_fn)?;

    let rgba_fn = lua.create_function(|lua, (r, g, b, a): (f64, f64, f64, Option<f64>)| {
        let ct = lua.create_table()?;
        ct.set(lua_color::R, r as f32)?;
        ct.set(lua_color::G, g as f32)?;
        ct.set(lua_color::B, b as f32)?;
        ct.set(lua_color::A, a.unwrap_or(1.0) as f32)?;
        Ok(ct)
    })?;
    color_tbl.set(lua_color::RGBA, rgba_fn)?;

    lua.globals().set(lua_color::COLOR, color_tbl)?;
    Ok(())
}

fn hex_to_f32(hex: &str, offset: usize, len: usize) -> mlua::Result<f32> {
    let s = &hex[offset..offset + len];
    let val = u32::from_str_radix(s, 16)
        .map_err(|_| mlua::Error::RuntimeError(format!("invalid hex digits: {s}")))?;
    let max = (1u32 << (len * 4)) - 1;
    Ok(val as f32 / max as f32)
}
