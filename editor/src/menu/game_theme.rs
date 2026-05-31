use crate::with_lua;
use engine_core::constants::extensions;
use engine_core::prelude::*;
use widgets::theme::Theme;

/// Returns a list of theme names discovered in `Resources/themes/`.
pub fn discover_themes() -> Vec<String> {
    let themes_dir = themes_folder();
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == extensions::LUA) {
                if let Some(name) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
                {
                    names.push(name);
                }
            }
        }
    }
    names.sort();
    names
}

/// Loads a theme from a Lua file in `Resources/themes/<name>.lua`.
pub fn load_theme(name: &str) -> Option<Theme> {
    let path = themes_folder().join(format!("{name}.{}", extensions::LUA));
    let src = std::fs::read_to_string(&path).ok()?;
    with_lua(|lua| {
        let tbl = lua.load(&src).set_name(name).eval::<mlua::Table>().ok()?;
        lua_table_to_theme(&tbl).ok()
    })
}
