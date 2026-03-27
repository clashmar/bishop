// lua_api_gen/src/main.rs
use engine_core::scripting::lua_constants::LUA_OWNER_SHARED_ENGINE;
use engine_core::scripting::modules::lua_module::*;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::io::Write;
use std::env;
use std::fs;
use game_lib as _;

fn main() {
    let out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .join("editor")
        .join("scripts")
        .join("_engine");

    fs::create_dir_all(&out_dir).unwrap();

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
            writeln!(file, "{LUA_OWNER_SHARED_ENGINE}").unwrap();
            writeln!(file, "---@meta").unwrap();
            writeln!(file).unwrap();
        }

        file.write_all(content.as_bytes()).unwrap();
        println!("Written to: {}", path.display());
    }
}
