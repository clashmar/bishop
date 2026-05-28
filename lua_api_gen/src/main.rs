// lua_api_gen/src/main.rs
mod menus_lua;

use engine_core::constants::paths;
use engine_core::scripting::lua_constants::{lua_dirs, lua_files, lua_ownership};
use engine_core::scripting::modules::lua_module::*;
use game_lib as _;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    let workspace_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .to_path_buf();
    let out_dirs = [
        workspace_root
            .join("editor")
            .join("scripts")
            .join("_engine"),
        workspace_root
            .join("games")
            .join("Demo")
            .join("Resources")
            .join("scripts")
            .join("_engine"),
    ];

    for out_dir in &out_dirs {
        fs::create_dir_all(out_dir).unwrap();
    }

    // Collect all generated snippets per target file
    let mut per_file: HashMap<&'static str, String> = HashMap::new();

    for reg in inventory::iter::<LuaApiRegistry> {
        let module = (reg.ctor)();
        let mut writer = LuaApiWriter::default();
        module.emit_api(&mut writer);

        // Append the snippet to the buffer for this file
        per_file
            .entry(reg.filename)
            .and_modify(|buf| buf.push_str(&writer.buf))
            .or_insert_with(|| writer.buf);
    }

    for out_dir in out_dirs {
        write_generated_files(&out_dir, &per_file);
    }

    write_menu_constants_files(&workspace_root);

    // Generate theme reference markdown
    write_theme_reference(&workspace_root);
}

fn write_generated_files(out_dir: &Path, per_file: &HashMap<&'static str, String>) {
    // Delete previous versions
    for filename in per_file.keys() {
        let path = out_dir.join(filename);
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    // Write (or append) each file
    for (filename, content) in per_file {
        let path = out_dir.join(filename);

        // If the file already exists append to it
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap();

        // Prepend the header
        if file.metadata().unwrap().len() == 0 {
            writeln!(file, "-- Auto-generated. Do not edit.").unwrap();
            writeln!(file, "{}", lua_ownership::LUA_OWNER_SHARED_ENGINE).unwrap();
            writeln!(file, "---@meta").unwrap();
            writeln!(file).unwrap();
        }

        file.write_all(content.as_bytes()).unwrap();
        println!("Written to: {}", path.display());
    }
}

fn write_menu_constants_files(workspace_root: &Path) {
    let menus_dir = workspace_root.join(paths::GAME_SAVE_ROOT).join("Demo").join(paths::RESOURCES_FOLDER).join(paths::MENUS_FOLDER);
    let content = menus_lua::generate_menus_lua_from_dir(&menus_dir)
        .unwrap_or_else(|err| panic!("{err}"));
    let out_dirs = [
        workspace_root.join("editor").join(paths::SCRIPTS_FOLDER).join(lua_dirs::ENGINE),
        workspace_root
            .join(paths::GAME_SAVE_ROOT)
            .join("Demo")
            .join(paths::RESOURCES_FOLDER)
            .join(paths::SCRIPTS_FOLDER)
            .join(lua_dirs::ENGINE),
    ];

    for out_dir in out_dirs {
        let path = out_dir.join(lua_files::MENUS);
        fs::write(&path, &content).unwrap();
        println!("Written to: {}", path.display());
    }
}

fn write_theme_reference(workspace_root: &Path) {
    use engine_core::scripting::lua_constants::lua_docs;

    let markdown = engine_core::theme::generate_theme_reference_markdown();
    let path = workspace_root
        .join(lua_docs::DOCS_DIR)
        .join(lua_docs::THEME_REFERENCE);
    fs::write(&path, &markdown).unwrap();
    println!(
        "cargo:warning=Theme reference written to {}",
        path.display()
    );
}

#[cfg(test)]
#[path = "tests/menus_lua_tests.rs"]
mod menus_lua_tests;
