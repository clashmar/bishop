use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use engine_core::constants::paths;

use super::{
    LUA_WASM_CXXFLAGS, WEB_WASM_TARGET, cargo_command, demo_resources_dir, run_checked,
    tool_available, wasm_target_installed,
};

const WEB_SHELL_PACKAGE: &str = "web_shell";
const WEB_SHELL_JS: &str = "web_shell.js";
const WEB_SHELL_WASM: &str = "web_shell.wasm";
const WEB_SHELL_INDEX: &str = "index.html";
const WEB_SHELL_INPUT: &str = "input.js";
const WEB_SHELL_AUDIO: &str = "audio.js";
const PLAYABLE_CXXFLAG: &str = "-fwasm-exceptions";

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Export {
    Shell,
    Game,
}

pub(super) fn build_web_shell() -> ExitCode {
    build_web(Export::Shell)
}

pub(super) fn build_web_game() -> ExitCode {
    build_web(Export::Game)
}

fn build_web(variant: Export) -> ExitCode {
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
    let mut build_command = cargo_command(&[
        "build", "-p", WEB_SHELL_PACKAGE, "--target", WEB_WASM_TARGET, "--locked",
    ]);
    let (build_dir, stage_dir, build_label) = match variant {
        Export::Shell => (
            web_shell_build_dir(&root),
            web_shell_stage_dir(&root),
            "web shell wasm build",
        ),
        Export::Game => {
            build_command.args(["--features", "playable-fixture"]);
            build_command.arg("--target-dir").arg(web_game_target_dir(&root));
            // Luau's C++ objects must use the same Wasm EH ABI as the Rust Emscripten link.
            // The separate target dir prevents reuse of objects built with -fexceptions.
            build_command.env(LUA_WASM_CXXFLAGS, PLAYABLE_CXXFLAG);
            (
                web_game_build_dir(&root),
                web_game_stage_dir(&root),
                "web game wasm build",
            )
        }
    };
    if !run_checked(build_command, build_label) {
        return ExitCode::FAILURE;
    }

    if variant == Export::Game && stage_dir.exists() {
        if let Err(error) = fs::remove_dir_all(&stage_dir) {
            eprintln!("failed to clear {}: {error}", stage_dir.display());
            return ExitCode::FAILURE;
        }
    }
    if let Err(error) = fs::create_dir_all(&stage_dir) {
        eprintln!("failed to create {}: {error}", stage_dir.display());
        return ExitCode::FAILURE;
    }

    let artifacts = [
        (
            root.join("web_shell").join(WEB_SHELL_INDEX),
            stage_dir.join(WEB_SHELL_INDEX),
        ),
        (
            root.join("web_shell").join(WEB_SHELL_INPUT),
            stage_dir.join(WEB_SHELL_INPUT),
        ),
        (
            root.join("web_shell").join(WEB_SHELL_AUDIO),
            stage_dir.join(WEB_SHELL_AUDIO),
        ),
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

    if variant == Export::Game {
        let fixture_urls = std::iter::once(web_shell::FIXTURE_MANIFEST_PATH.to_owned()).chain(
            web_shell::FIXTURE_MODULES
                .iter()
                .map(|name| web_shell::fixture_module_path(name)),
        );
        for url in fixture_urls {
            let source = match fixture_source(&root, &url) {
                Ok(source) => source,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            if !stage_file(&source, &stage_dir.join(&url), "web fixture resource") {
                return ExitCode::FAILURE;
            }
        }
    }

    match variant {
        Export::Shell => println!("web shell staged at {}", stage_dir.display()),
        Export::Game => println!("playable web fixture staged at {}", stage_dir.display()),
    }
    println!("serve it with:");
    println!("  python3 -m http.server --directory {} 8000", stage_dir.display());
    println!("then open:");
    println!("  http://localhost:8000/");
    ExitCode::SUCCESS
}

fn web_shell_build_dir(root: &Path) -> PathBuf {
    root.join("target").join(WEB_WASM_TARGET).join("debug")
}

fn web_game_target_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-game-build")
}

fn web_game_build_dir(root: &Path) -> PathBuf {
    web_game_target_dir(root).join(WEB_WASM_TARGET).join("debug")
}

fn web_shell_stage_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-shell")
}

fn web_game_stage_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-game")
}

fn fixture_source(root: &Path, asset_url: &str) -> Result<PathBuf, String> {
    let relative = asset_url
        .strip_prefix(web_shell::FIXTURE_ASSET_ROOT)
        .ok_or_else(|| {
            format!(
                "fixture URL outside {}: {asset_url}",
                web_shell::FIXTURE_ASSET_ROOT
            )
        })?;
    Ok(root
        .join(paths::GAME_SAVE_ROOT)
        .join(web_shell::FIXTURE_NAME)
        .join(paths::RESOURCES_FOLDER)
        .join(relative))
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
