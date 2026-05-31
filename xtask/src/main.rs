use engine_core::constants::paths;
use engine_core::scripting::lua_constants::lua_files;
use engine_core::scripting::lua_project::{
    scaffold_luacheckrc, scaffold_luarc_json, scaffold_stylua_toml, workspace_luacheckrc,
    workspace_luarc_json, workspace_stylua_toml,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn demo_root(root: &Path) -> PathBuf {
    root.join(paths::GAME_SAVE_ROOT).join(paths::DEMO_GAME)
}

fn demo_scripts_dir(root: &Path) -> PathBuf {
    demo_root(root)
        .join(paths::RESOURCES_FOLDER)
        .join(paths::SCRIPTS_FOLDER)
}

fn write_if_changed(path: &Path, contents: &str) {
    if fs::read_to_string(path).ok().as_deref() == Some(contents) {
        return;
    }
    fs::write(path, contents).expect("failed to sync Lua config file");
}

fn sync_lua_project_files(root: &Path) {
    write_if_changed(&root.join(lua_files::LUARC), &workspace_luarc_json());
    write_if_changed(&root.join(lua_files::LUACHECK), &workspace_luacheckrc());
    write_if_changed(&root.join(lua_files::STYLUA), &workspace_stylua_toml());

    let demo_root = demo_root(root);
    write_if_changed(&demo_root.join(lua_files::LUARC), &scaffold_luarc_json());
    write_if_changed(&demo_root.join(lua_files::LUACHECK), &scaffold_luacheckrc());
    write_if_changed(&demo_root.join(lua_files::STYLUA), &scaffold_stylua_toml());
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo xtask <subcommand>");
        return ExitCode::FAILURE;
    }

    match args[1].as_str() {
        "check-lua" => check_lua(),
        other => {
            eprintln!("Unknown subcommand: {other}");
            eprintln!("Available: check-lua");
            ExitCode::FAILURE
        }
    }
}

fn check_lua() -> ExitCode {
    let mut failed = false;
    let root = std::env::current_dir().expect("xtask must run from workspace root");
    sync_lua_project_files(&root);

    let luacheck_config = root.join(lua_files::LUACHECK);
    let demo_scripts = demo_scripts_dir(&root);
    match Command::new("luacheck")
        .arg(demo_scripts)
        .arg("--config")
        .arg(&luacheck_config)
        .arg("--no-color")
        .status()
    {
        Ok(status) if status.success() => println!("luacheck: PASS"),
        Ok(_) => {
            eprintln!("luacheck: FAIL (see warnings above)");
            failed = true;
        }
        Err(e) => {
            eprintln!("luacheck: NOT FOUND ({e})");
            failed = true;
        }
    }

    let stylua_files = demo_lua_files(&root);
    let mut stylua = Command::new("stylua");
    stylua.arg("--config-path");
    stylua.arg(root.join(lua_files::STYLUA));
    stylua.arg("--check");
    for file in &stylua_files {
        stylua.arg(file);
    }

    match stylua.status() {
        Ok(status) if status.success() => println!("stylua: PASS"),
        Ok(_) => {
            eprintln!("stylua: FAIL (formatting differences found)");
            failed = true;
        }
        Err(e) => {
            eprintln!("stylua: NOT FOUND ({e})");
            failed = true;
        }
    }

    let config_path = root.join(".luarc.json");
    let mut luals = Command::new("lua-language-server");
    luals.arg(format!("--configpath={}", config_path.display()));
    luals.arg("--check=.");
    luals.arg("--check_format=pretty");

    match luals.status() {
        Ok(status) if status.success() => println!("lua-language-server: PASS"),
        Ok(_) => {
            eprintln!("lua-language-server: FAIL (type errors found)");
            failed = true;
        }
        Err(e) => {
            eprintln!("lua-language-server: NOT FOUND ({e})");
            failed = true;
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        println!("All Lua checks passed.");
        ExitCode::SUCCESS
    }
}

fn demo_lua_files(root: &Path) -> Vec<String> {
    let scripts_dir = demo_scripts_dir(root);
    let mut files: Vec<String> = fs::read_dir(&scripts_dir)
        .expect("Demo scripts directory should exist")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "lua"))
        .map(|path| path.display().to_string())
        .collect();
    files.sort();
    files
}
