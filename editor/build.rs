// editor/build.rs
use engine_core::constants::paths;
use engine_core::ecs::component_registry::public_lua_components;
use engine_core::input::input_table::*;
use engine_core::scripting::lua_constants::{lua_dirs, lua_files, lua_ownership};
use engine_core::scripting::lua_project::{engine_relative_path, generate_globals_lua};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const SRC_DIR: &str = "src";
const EDITOR_ASSETS_DIR: &str = "editor_assets";

fn shared_engine_dir(manifest_dir: &Path) -> PathBuf {
    manifest_dir.join(paths::SCRIPTS_FOLDER).join(lua_dirs::ENGINE)
}

fn write_engine_file(out_dir: &Path, filename: &str, contents: &str) {
    let target = out_dir.join(engine_relative_path(filename));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("cannot create grouped _engine folder");
    }
    write_if_changed(&target, contents);
}

fn demo_engine_dir(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join(paths::GAME_SAVE_ROOT)
        .join(paths::DEMO_GAME)
        .join(paths::RESOURCES_FOLDER)
        .join(paths::SCRIPTS_FOLDER)
        .join(lua_dirs::ENGINE)
}

fn main() -> std::io::Result<()> {
    generate_lua_script();
    generate_lua_components();
    generate_lua_direction();
    generate_lua_input();
    generate_lua_globals();
    generate_engine_scripts_rs();

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set("FileVersion", "1.0.0.0")
            .set_icon("windows/Icon.ico")
            .set(
                "FileDescription",
                "Bishop Engine: a cross platform 2dD editor.",
            )
            .set("ProductVersion", "1.0.0.0")
            .set("ProductName", "Bishop Engine")
            .set("OriginalFilename", "Bishop.exe")
            .set("LegalCopyright", "© 2025 Clashmar")
            .set("LegalTrademark", "Bishop Engine™")
            .set("CompanyName", "Clashmar Ltd.")
            .set("Comments", "Lightweight 2D Editor")
            .set("InternalName", "Bishop Engine")
            .set_version_info(winres::VersionInfo::FILEVERSION, 0x0001000000000000)
            .set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001000000000000);

        res.compile()?;
    }
    Ok(())
}

fn write_if_changed(target: &Path, contents: &str) {
    let existing = fs::read_to_string(target).ok();
    if existing.as_deref() == Some(contents) {
        return;
    }

    fs::write(target, contents).unwrap_or_else(|_| panic!("Cannot write {}", target.display()));
    println!("cargo:warning=generated {}", target.display());
}

fn generate_lua_components() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .to_path_buf();

    let mut lua = format!(
        "-- Auto-generated. Do not edit.\n\
        {}\n\
        ---@meta\n\
        ---@alias vec2 {{ x: number, y: number }}\n\
        ---@alias vec3 {{ x: number, y: number, z: number }}\n\n",
        lua_ownership::LUA_OWNER_SHARED_ENGINE,
    );

    // Generate class definitions for each component with their schema
    for reg in public_lua_components() {
        let schema = (reg.lua_schema)();

        // Check if this is an alias type (single-value tuple struct)
        if schema.len() == 1 && schema[0].0 == "__alias__" {
            lua.push_str(&format!("---@alias {} {}\n\n", reg.type_name, schema[0].1));
            continue;
        }

        if schema.is_empty() {
            // For marker/unit structs, emit a description above the class annotation.
            lua.push_str("--- Marker component\n");
        }

        // Generate a class definition
        lua.push_str(&format!("---@class {}\n", reg.type_name));

        if !schema.is_empty() {
            // Add field annotations from the schema
            for (field_name, field_type) in schema {
                lua.push_str(&format!("---@field {} {}\n", field_name, field_type));
            }
        }

        lua.push('\n');
    }

    // Generate the ComponentId class with all component names
    lua.push_str("---@class ComponentId\n");
    for reg in public_lua_components() {
        lua.push_str(&format!(
            "---@field {} \"{}\"\n",
            reg.type_name, reg.type_name
        ));
    }
    lua.push('\n');

    lua.push_str("local C = {}\n\n");

    // Fill table assignments
    for reg in public_lua_components() {
        lua.push_str(&format!("C.{} = \"{}\"\n", reg.type_name, reg.type_name));
    }

    lua.push_str("\nreturn C\n");

    let out_dirs = [shared_engine_dir(&manifest_dir), demo_engine_dir(&workspace_root)];
    for out_dir in out_dirs {
        fs::create_dir_all(&out_dir).expect("cannot create _engine folder");
        write_engine_file(&out_dir, lua_files::COMPONENTS, &lua);
    }
}

fn generate_lua_input() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .to_path_buf();

    let mut lua = format!(
        "-- Auto-generated. Do not edit.\n\
        {}\n\
        ---@meta\n\n",
        lua_ownership::LUA_OWNER_SHARED_ENGINE,
    );

    // Enum definition
    lua.push_str("---@enum Input\nlocal Input = {\n");

    // Avoids duplicates
    let mut seen = HashSet::new();

    // Keyboard
    for &(name, _code) in KEY_TABLE.iter() {
        // Nmae is the literal string that should be used at runtime
        if seen.insert(name) {
            let key = lua_key_name(name);
            lua.push_str(&format!("    {} = \"{}\",\n", key, name));
        }
    }

    // Mouse
    for &(name, _code) in MOUSE_TABLE.iter() {
        if seen.insert(name) {
            let key = lua_key_name(name);
            lua.push_str(&format!("    {} = \"{}\",\n", key, name));
        }
    }

    lua.push_str("}\n");

    // Return the enum table
    lua.push_str("\nreturn Input\n");

    // Write the file
    let out_dirs = [shared_engine_dir(&manifest_dir), demo_engine_dir(&workspace_root)];
    for out_dir in out_dirs {
        fs::create_dir_all(&out_dir).expect("cannot create _engine folder");
        write_engine_file(&out_dir, lua_files::INPUT, &lua);
    }
}

fn generate_lua_direction() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .to_path_buf();

    let lua = format!(
        "-- Auto-generated. Do not edit.\n\
        {}\n\
        ---@meta\n\n\
        ---@enum Direction\n\
        local Direction = {{\n\
            Up = \"up\",\n\
            Down = \"down\",\n\
            Left = \"left\",\n\
            Right = \"right\",\n\
            UpLeft = \"up_left\",\n\
            UpRight = \"up_right\",\n\
            DownLeft = \"down_left\",\n\
            DownRight = \"down_right\",\n\
        }}\n\n\
        return Direction\n",
        lua_ownership::LUA_OWNER_SHARED_ENGINE,
    );

    let out_dirs = [shared_engine_dir(&manifest_dir), demo_engine_dir(&workspace_root)];
    for out_dir in out_dirs {
        fs::create_dir_all(&out_dir).expect("cannot create _engine folder");
        write_engine_file(&out_dir, lua_files::DIRECTION, &lua);
    }
}

fn generate_lua_globals() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .to_path_buf();
    let out_dirs = [shared_engine_dir(&manifest_dir), demo_engine_dir(&workspace_root)];
    let lua = generate_globals_lua();

    for out_dir in out_dirs {
        fs::create_dir_all(&out_dir).expect("cannot create _engine folder");
        let target = out_dir.join(lua_files::GLOBALS);
        write_if_changed(&target, &lua);
    }
}

fn generate_lua_script() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .to_path_buf();
    let out_dirs = [shared_engine_dir(&manifest_dir), demo_engine_dir(&workspace_root)];

    let lua = format!(
        "-- Auto-generated. Do not edit.\n\
        {}\n\
        ---@meta\n\
        ---@class Script\n\
        ---@field public table\n\
        ---@field entity Entity\n\
        ---@field update fun(self: Script, dt: number)\n\
        ---@field init fun(self: Script, init?: table)\n\
        ---@field interact fun(self: Script)\n\
        ---@field on_exit fun(self: Script)\n\
        local Script = {{}}\n\
        return Script\n",
        lua_ownership::LUA_OWNER_SHARED_ENGINE,
    );

    for out_dir in out_dirs {
        fs::create_dir_all(&out_dir).expect("cannot create _engine folder");
        write_engine_file(&out_dir, lua_files::SCRIPT, &lua);
    }
}

fn collect_engine_script_entries(engine_dir: &Path, current_dir: &Path, entries: &mut Vec<String>) {
    if let Ok(dir) = fs::read_dir(current_dir) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_engine_script_entries(engine_dir, &path, entries);
                continue;
            }
            if !path.extension().map(|e| e == "lua").unwrap_or(false) {
                continue;
            }
            let relative = path
                .strip_prefix(engine_dir)
                .expect("engine script should live under _engine")
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(relative);
        }
    }
}

/// Generates Rust code that embeds all .lua files from the _engine directory.
fn generate_engine_scripts_rs() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let engine_dir = shared_engine_dir(&manifest_dir);
    let out_dir = manifest_dir.join(SRC_DIR).join(EDITOR_ASSETS_DIR);

    // Collect all .lua files recursively.
    let mut entries: Vec<String> = Vec::new();
    collect_engine_script_entries(&engine_dir, &engine_dir, &mut entries);
    entries.sort();

    // Generate Rust code
    let mut rust = String::from(
        "// Auto-generated by build.rs. Do not edit.\n\n\
         /// Embedded _engine Lua scripts for new game projects.\n\
         pub static ENGINE_SCRIPTS: &[(&str, &str)] = &[\n",
    );

    for relative_path in &entries {
        rust.push_str(&format!(
            "    (\"{}\", include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/scripts/_engine/{}\"))),\n",
            relative_path, relative_path
        ));
    }

    rust.push_str("];\n");

    let target = out_dir.join("engine_scripts.rs");
    write_if_changed(&target, &rust);

    // Rerun if _engine directory changes
    println!(
        "cargo:rerun-if-changed={}/{}",
        paths::SCRIPTS_FOLDER,
        lua_dirs::ENGINE
    );

    // Rerun if bishop theme file changes
    let demo_theme_path = PathBuf::from("..")
        .join(paths::GAME_SAVE_ROOT)
        .join(paths::DEMO_GAME)
        .join(paths::RESOURCES_FOLDER)
        .join(paths::THEMES_FOLDER)
        .join(lua_files::BISHOP_THEME);
    println!("cargo:rerun-if-changed={}", demo_theme_path.display());
}
