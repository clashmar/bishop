use crate::scripting::lua_constants::{lua_dirs, lua_files, lua_globals, lua_ownership};
use serde_json::json;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaGlobalModule {
    pub filename: &'static str,
    pub global_name: &'static str,
}

pub const LUA_RUNTIME_GLOBALS: &[&str] = &[
    lua_globals::ENGINE,
    lua_globals::COLOR,
    lua_globals::WIDGET,
    lua_globals::PRIVATE,
    lua_globals::LOCAL,
];

pub const LUA_GLOBAL_MODULES: &[LuaGlobalModule] = &[
    LuaGlobalModule {
        filename: lua_files::INPUT,
        global_name: lua_globals::INPUT,
    },
    LuaGlobalModule {
        filename: lua_files::DIRECTION,
        global_name: lua_globals::DIRECTION,
    },
    LuaGlobalModule {
        filename: lua_files::COMPONENTS,
        global_name: lua_globals::COMPONENTS,
    },
    LuaGlobalModule {
        filename: lua_files::ANIMATIONS,
        global_name: lua_globals::ANIMATIONS,
    },
    LuaGlobalModule {
        filename: lua_files::PREFABS,
        global_name: lua_globals::PREFABS,
    },
    LuaGlobalModule {
        filename: lua_files::SOUNDS,
        global_name: lua_globals::SOUNDS,
    },
    LuaGlobalModule {
        filename: lua_files::MENUS,
        global_name: lua_globals::MENUS,
    },
    LuaGlobalModule {
        filename: lua_files::SCRIPT,
        global_name: lua_globals::SCRIPT,
    },
    LuaGlobalModule {
        filename: lua_files::ENTITY,
        global_name: lua_globals::ENTITY,
    },
];

pub fn all_known_globals() -> Vec<&'static str> {
    let mut globals = LUA_RUNTIME_GLOBALS.to_vec();
    globals.extend(LUA_GLOBAL_MODULES.iter().map(|module| module.global_name));
    globals
}

pub fn engine_relative_path(filename: &str) -> PathBuf {
    match filename {
        lua_files::GLOBALS => PathBuf::from(lua_files::GLOBALS),
        lua_files::INPUT
        | lua_files::DIRECTION
        | lua_files::COMPONENTS
        | lua_files::ANIMATIONS
        | lua_files::PREFABS
        | lua_files::SOUNDS
        | lua_files::MENUS => PathBuf::from(lua_dirs::DATA).join(filename),
        lua_files::ENGINE
        | lua_files::ENTITY
        | lua_files::SCRIPT
        | lua_files::AUDIO
        | lua_files::SAVE
        | lua_files::TEXT => PathBuf::from(lua_dirs::RUNTIME).join(filename),
        lua_files::MENU | lua_files::THEME | lua_files::COLOR => {
            PathBuf::from(lua_dirs::UI).join(filename)
        }
        _ => panic!("unknown engine Lua filename: {filename}"),
    }
}

pub fn engine_require_path(filename: &str) -> String {
    let stem = filename
        .strip_suffix(".lua")
        .expect("engine Lua module filenames must end with .lua");

    match filename {
        lua_files::INPUT
        | lua_files::DIRECTION
        | lua_files::COMPONENTS
        | lua_files::ANIMATIONS
        | lua_files::PREFABS
        | lua_files::SOUNDS
        | lua_files::MENUS => format!("{}.{}.{}", lua_dirs::ENGINE, lua_dirs::DATA, stem),
        lua_files::ENGINE
        | lua_files::ENTITY
        | lua_files::SCRIPT
        | lua_files::AUDIO
        | lua_files::SAVE
        | lua_files::TEXT => format!("{}.{}.{}", lua_dirs::ENGINE, lua_dirs::RUNTIME, stem),
        lua_files::MENU | lua_files::THEME | lua_files::COLOR => {
            format!("{}.{}.{}", lua_dirs::ENGINE, lua_dirs::UI, stem)
        }
        _ => panic!("unknown engine Lua filename: {filename}"),
    }
}

pub fn generate_globals_lua() -> String {
    let mut lua = format!(
        "-- Auto-generated. Do not edit.\n{}\n---@meta\n\n",
        lua_ownership::LUA_OWNER_SHARED_ENGINE,
    );

    for module in LUA_GLOBAL_MODULES {
        lua.push_str(&format!(
            "{} = require(\"{}\")\n",
            module.global_name,
            engine_require_path(module.filename),
        ));
    }

    lua
}

pub fn scaffold_luarc_json() -> String {
    serde_json::to_string_pretty(&json!({
        "$schema": "https://raw.githubusercontent.com/LuaLS/vscode-lua/master/setting/schema.json",
        "diagnostics": {
            "globals": all_known_globals(),
        },
    }))
    .expect("scaffold luarc should serialize")
        + "\n"
}

pub fn workspace_luarc_json() -> String {
    serde_json::to_string_pretty(&json!({
        "$schema": "https://raw.githubusercontent.com/LuaLS/vscode-lua/master/setting/schema.json",
        "workspace": {
            "ignoreDir": [
                "editor/scripts/_engine",
                "games/*/Resources/scripts/_engine",
                "!games/Demo/Resources/scripts/_engine",
            ],
        },
        "diagnostics": {
            "globals": all_known_globals(),
        },
    }))
    .expect("workspace luarc should serialize")
        + "\n"
}

pub fn scaffold_luacheckrc() -> String {
    generate_luacheckrc("Resources/scripts/_engine/**/*.lua")
}

pub fn workspace_luacheckrc() -> String {
    generate_luacheckrc("games/Demo/Resources/scripts/_engine/**/*.lua")
}

fn generate_luacheckrc(engine_glob: &str) -> String {
    let top_level_globals = all_known_globals()
        .into_iter()
        .map(|name| format!("    \"{name}\","))
        .collect::<Vec<_>>()
        .join("\n");
    let nested_globals = all_known_globals()
        .into_iter()
        .map(|name| format!("        \"{name}\","))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "std = \"lua54\"\n\nglobals = {{\n{top_level_globals}\n}}\n\nunused_args = false\n\nfiles[\"{engine_glob}\"] = {{\n    globals = {{\n{nested_globals}\n    }},\n    unused_args = false,\n    ignore = {{ \"211\", \"631\" }},\n}}\n"
    )
}

pub fn scaffold_stylua_toml() -> String {
    [
        "column_width = 100",
        "indent_type = \"Spaces\"",
        "indent_width = 4",
        "quote_style = \"AutoPreferDouble\"",
        "call_parentheses = \"Always\"",
        "",
    ]
    .join("\n")
}

pub fn workspace_stylua_toml() -> String {
    scaffold_stylua_toml()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globals_prelude_exports_agreed_namespace_tables() {
        let lua = generate_globals_lua();

        for expected in [
            "Input = require(\"_engine.data.input\")",
            "Direction = require(\"_engine.data.direction\")",
            "Components = require(\"_engine.data.components\")",
            "Animations = require(\"_engine.data.animations\")",
            "Prefabs = require(\"_engine.data.prefabs\")",
            "Sounds = require(\"_engine.data.sounds\")",
            "Menus = require(\"_engine.data.menus\")",
            "Script = require(\"_engine.runtime.script\")",
            "Entity = require(\"_engine.runtime.entity\")",
        ] {
            assert!(lua.contains(expected), "missing global line: {expected}\n{lua}");
        }
    }

    #[test]
    fn scaffold_luarc_lists_runtime_and_generated_globals() {
        let luarc = scaffold_luarc_json();

        for expected in [
            "engine",
            "Color",
            "Widget",
            "Input",
            "Direction",
            "Components",
            "Animations",
            "Prefabs",
            "Sounds",
            "Menus",
            "Script",
            "Entity",
        ] {
            assert!(
                luarc.contains(expected),
                "missing luarc global: {expected}\n{luarc}"
            );
        }
    }

    #[test]
    fn scaffold_luacheckrc_lists_runtime_and_generated_globals() {
        let luacheckrc = scaffold_luacheckrc();

        for expected in [
            "\"engine\"",
            "\"Color\"",
            "\"Widget\"",
            "\"Input\"",
            "\"Direction\"",
            "\"Components\"",
            "\"Animations\"",
            "\"Prefabs\"",
            "\"Sounds\"",
            "\"Menus\"",
            "\"Script\"",
            "\"Entity\"",
        ] {
            assert!(
                luacheckrc.contains(expected),
                "missing luacheck global: {expected}\n{luacheckrc}"
            );
        }
    }
}
