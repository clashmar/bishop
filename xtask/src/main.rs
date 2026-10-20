use engine_core::constants::paths;
use engine_core::scripting::lua_constants::{lua_dirs, lua_files};
use engine_core::scripting::lua_project::{
    engine_relative_path, scaffold_luacheckrc, scaffold_luarc_json, scaffold_stylua_toml,
    workspace_luacheckrc, workspace_luarc_json, workspace_stylua_toml,
};
use engine_core::scripting::{
    collect_entry_handles, collect_world_handles, generate_entries_lua, generate_worlds_lua,
};
use engine_core::storage::load_full_game_from_folder;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const WEB_WASM_TARGET: &str = "wasm32-unknown-emscripten";
const LUA_WASM_TARGET: &str = "wasm32-unknown-emscripten";
const LUA_WASM_CXXFLAGS: &str = "CXXFLAGS_wasm32_unknown_emscripten";
const LUA_WASM_REQUIRED_CXXFLAG: &str = "-fexceptions";
const LUA_BOOTSTRAP_TEST: &str = "register_runtime_modules_supports_editor_script_load_patterns";
const WEB_SHELL_PACKAGE: &str = "web_shell";
const WEB_SHELL_JS: &str = "web_shell.js";
const WEB_SHELL_WASM: &str = "web_shell.wasm";
const WEB_SHELL_INDEX: &str = "index.html";
const WEB_SHELL_INPUT: &str = "input.js";

fn demo_root(root: &Path) -> PathBuf {
    root.join(paths::GAME_SAVE_ROOT).join(paths::DEMO_GAME)
}

fn demo_resources_dir(root: &Path) -> PathBuf {
    demo_root(root).join(paths::RESOURCES_FOLDER)
}

fn demo_scripts_dir(root: &Path) -> PathBuf {
    demo_resources_dir(root).join(paths::SCRIPTS_FOLDER)
}

fn write_if_changed(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent directories");
    }
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
        "build-web-shell" => build_web_shell(),
        "check-lua" => check_lua(),
        "check-lua-wasm" => check_lua_wasm(),
        "check-web" => check_web(),
        "regen-demo-data" => regen_demo_data(),
        other => {
            eprintln!("Unknown subcommand: {other}");
            eprintln!("Available: build-web-shell, check-lua, check-lua-wasm, check-web, regen-demo-data");
            ExitCode::FAILURE
        }
    }
}

fn regen_demo_data() -> ExitCode {
    let root = std::env::current_dir().expect("xtask must run from workspace root");
    let resources_dir = demo_resources_dir(&root);

    let game = match load_full_game_from_folder(&resources_dir) {
        Ok(game) => game,
        Err(error) => {
            eprintln!("Failed to load Demo game from {}: {error}", resources_dir.display());
            return ExitCode::FAILURE;
        }
    };

    let worlds = collect_world_handles(&game);
    let entries = collect_entry_handles(&game);
    let scripts_dir = demo_scripts_dir(&root);

    write_if_changed(
        &scripts_dir.join(lua_dirs::ENGINE).join(engine_relative_path(lua_files::WORLDS)),
        &generate_worlds_lua(&worlds),
    );
    write_if_changed(
        &scripts_dir.join(lua_dirs::ENGINE).join(engine_relative_path(lua_files::ENTRIES)),
        &generate_entries_lua(&entries),
    );

    println!("Regenerated Demo worlds.lua and entries.lua.");
    ExitCode::SUCCESS
}

fn check_lua_wasm() -> ExitCode {
    if !wasm_target_installed(LUA_WASM_TARGET) {
        eprintln!("{LUA_WASM_TARGET} target is not installed.");
        eprintln!("Install it with: rustup target add {LUA_WASM_TARGET}");
        return ExitCode::FAILURE;
    }

    let native_ok = run_checked(
        cargo_command(&[
            "test",
            "-p",
            "engine_core",
            "--locked",
            LUA_BOOTSTRAP_TEST,
        ]),
        "shared lua bootstrap native proof",
    );

    let mut wasm_command = cargo_command(&[
        "check",
        "-p",
        "engine_core",
        "--target",
        LUA_WASM_TARGET,
        "--no-default-features",
        "--locked",
    ]);
    apply_lua_wasm_env(&mut wasm_command);

    let wasm_ok = run_checked(wasm_command, "shared lua wasm compile");

    if !wasm_ok {
        println!(
            "shared lua wasm compile: BLOCKED. Use the compiler output above as the Lua WASM blocker inventory."
        );
    }

    if native_ok && wasm_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn apply_lua_wasm_env(command: &mut Command) {
    let flags = std::env::var(LUA_WASM_CXXFLAGS).unwrap_or_default();
    if flags.split_whitespace().any(|flag| flag == LUA_WASM_REQUIRED_CXXFLAG) {
        return;
    }

    let flags = if flags.is_empty() {
        LUA_WASM_REQUIRED_CXXFLAG.to_string()
    } else {
        format!("{flags} {LUA_WASM_REQUIRED_CXXFLAG}")
    };
    command.env(LUA_WASM_CXXFLAGS, flags);
}

fn build_web_shell() -> ExitCode {
    if !wasm_target_installed(WEB_WASM_TARGET) {
        eprintln!("{WEB_WASM_TARGET} target is not installed.");
        eprintln!("Install it with: rustup target add {WEB_WASM_TARGET}");
        return ExitCode::FAILURE;
    }

    let mut missing_tools = Vec::new();
    for tool in ["emcc", "em++"] {
        if !tool_available(tool) {
            missing_tools.push(tool);
        }
    }
    if !missing_tools.is_empty() {
        eprintln!("Missing Emscripten tools: {}", missing_tools.join(", "));
        eprintln!("Install or activate Emscripten so emcc and em++ are on PATH.");
        return ExitCode::FAILURE;
    }

    let root = std::env::current_dir().expect("xtask must run from workspace root");
    let build_ok = run_checked(
        cargo_command(&[
            "build",
            "-p",
            WEB_SHELL_PACKAGE,
            "--target",
            WEB_WASM_TARGET,
            "--locked",
        ]),
        "web shell wasm build",
    );
    if !build_ok {
        return ExitCode::FAILURE;
    }

    let build_dir = web_shell_build_dir(&root);
    let stage_dir = web_shell_stage_dir(&root);
    if let Err(error) = fs::create_dir_all(&stage_dir) {
        eprintln!("failed to create {}: {error}", stage_dir.display());
        return ExitCode::FAILURE;
    }

    let artifacts = [
        (root.join("web_shell").join(WEB_SHELL_INDEX), stage_dir.join(WEB_SHELL_INDEX)),
        (root.join("web_shell").join(WEB_SHELL_INPUT), stage_dir.join(WEB_SHELL_INPUT)),
        (build_dir.join(WEB_SHELL_JS), stage_dir.join(WEB_SHELL_JS)),
        (build_dir.join(WEB_SHELL_WASM), stage_dir.join(WEB_SHELL_WASM)),
    ];

    for (source, dest) in artifacts {
        if !stage_file(&source, &dest, "web shell artifact") {
            return ExitCode::FAILURE;
        }
    }

    let demo_asset_file = Path::new(web_shell::DEMO_TITLE_ASSET_PATH)
        .file_name()
        .expect("Demo title browser asset path must have a filename");
    let demo_asset_source = demo_resources_dir(&root)
        .join(paths::TEXT_FOLDER)
        .join(paths::TEXT_LANGUAGE_FOLDER)
        .join(paths::UI_TEXT_FOLDER)
        .join(demo_asset_file);
    let demo_asset_dest = stage_dir.join(web_shell::DEMO_TITLE_ASSET_PATH);
    if !stage_file(&demo_asset_source, &demo_asset_dest, "web shell demo asset") {
        return ExitCode::FAILURE;
    }

    println!("web shell staged at {}", stage_dir.display());
    println!("serve it with:");
    println!("  python3 -m http.server --directory {} 8000", stage_dir.display());
    println!("then open:");
    println!("  http://localhost:8000/");
    ExitCode::SUCCESS
}

fn check_web() -> ExitCode {
    if !wasm_target_installed(WEB_WASM_TARGET) {
        eprintln!("{WEB_WASM_TARGET} target is not installed.");
        eprintln!("Install it with: rustup target add {WEB_WASM_TARGET}");
        return ExitCode::FAILURE;
    }

    let foundation_ok = run_checked(
        cargo_command(&[
            "check",
            "-p",
            "bishop",
            "--target",
            WEB_WASM_TARGET,
            "--no-default-features",
            "--locked",
        ]),
        "bishop web foundation",
    );

    let mut runtime_command = cargo_command(&[
        "check",
        "-p",
        "game",
        "--lib",
        "--target",
        WEB_WASM_TARGET,
        "--no-default-features",
        "--locked",
    ]);
    apply_lua_wasm_env(&mut runtime_command);

    let runtime_ok = run_checked(runtime_command, "game runtime web probe");

    if !runtime_ok {
        println!(
            "game runtime web probe: BLOCKED. This is expected during Chunk 3; use the compiler output above as the blocker inventory."
        );
    }

    if foundation_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn tool_available(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn wasm_target_installed(target: &str) -> bool {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();

    let output = match output {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            eprintln!("rustup target list --installed failed with {}", output.status);
            return false;
        }
        Err(error) => {
            eprintln!("rustup not found: {error}");
            return false;
        }
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.trim() == target)
}

fn web_shell_build_dir(root: &Path) -> PathBuf {
    root.join("target").join(WEB_WASM_TARGET).join("debug")
}

fn web_shell_stage_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-shell")
}

fn stage_file(source: &Path, dest: &Path, label: &str) -> bool {
    if !source.exists() {
        eprintln!("expected {label} missing: {}", source.display());
        return false;
    }

    let parent = dest
        .parent()
        .expect("staged file destination must have a parent");
    if let Err(error) = fs::create_dir_all(parent) {
        eprintln!("failed to create {}: {error}", parent.display());
        return false;
    }

    if let Err(error) = fs::copy(source, dest) {
        eprintln!(
            "failed to stage {} to {}: {error}",
            source.display(),
            dest.display()
        );
        return false;
    }

    true
}

fn cargo_command(args: &[&str]) -> Command {
    let mut command = Command::new("cargo");
    command.args(args);
    command
}

fn run_checked(mut command: Command, label: &str) -> bool {
    println!("{label}: RUN");
    match command.status() {
        Ok(status) if status.success() => {
            println!("{label}: PASS");
            true
        }
        Ok(status) => {
            eprintln!("{label}: FAIL ({status})");
            false
        }
        Err(error) => {
            eprintln!("{label}: NOT FOUND ({error})");
            false
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
