use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use engine_core::constants::paths;
use engine_core::storage::web_site_export::{INDEX_TEMPLATE_FILE, stage_web_site};

use super::{
    LUA_WASM_CXXFLAGS, WEB_WASM_TARGET, cargo_command, demo_resources_dir, run_checked,
    tool_available, wasm_target_installed,
};

const WEB_SHELL_PACKAGE: &str = "web_shell";
const WEB_SHELL_JS: &str = "web_shell.js";
const WEB_SHELL_WASM: &str = "web_shell.wasm";
const WEB_SHELL_INDEX: &str = "index.html";
const WEB_RENDERER_INDEX: &str = "renderer-proof.html";
const WEB_DEMO_INDEX: &str = "demo-checkpoint.html";
const WEB_PLAYER_HTML: &str = "player.html";
const WEB_PLAYER_CONFIG: &str = "player/config.mjs";
const WEB_PLAYER_STORE: &str = "player/save_store.mjs";
const WEB_PLAYER_TEMPLATE: &str = "embed-example.html";
const WEB_SHELL_INPUT: &str = "input.js";
const WEB_SHELL_AUDIO: &str = "audio.js";
const PLAYABLE_CXXFLAG: &str = "-fwasm-exceptions";

mod demo_manifest;
use demo_manifest::stage_demo_resources;
#[cfg(test)]
mod demo_manifest_tests;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Export {
    Shell,
    Game,
    Renderer,
    DemoCheckpoint,
    Player,
}

pub(super) fn build_web_shell() -> ExitCode {
    build_web(Export::Shell)
}

pub(super) fn build_web_game() -> ExitCode {
    build_web(Export::Game)
}

pub(super) fn build_web_renderer_proof() -> ExitCode {
    build_web(Export::Renderer)
}

pub(super) fn build_web_demo_checkpoint() -> ExitCode {
    build_web(Export::DemoCheckpoint)
}

pub(super) fn build_web_player_runtime(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("Usage: cargo xtask build-web-player-runtime");
        return ExitCode::FAILURE;
    }
    build_web(Export::Player)
}

pub(super) fn export_web_game(args: &[String]) -> ExitCode {
    let (resources, destination) = match parse_export_web_game_args(args) {
        Ok(paths) => paths,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let root = std::env::current_dir().expect("xtask must run from workspace root");
    let runtime = web_player_stage_dir(&root);
    if !runtime.is_dir() {
        eprintln!("prebuilt Bishop Player runtime missing at {}; run cargo xtask build-web-player-runtime first", runtime.display());
        return ExitCode::FAILURE;
    }
    match stage_web_site(&resources, &runtime, &destination) {
        Ok(layout) => {
            println!("Bishop web game site staged at {} (game: {})", destination.display(), layout.game_dir_name);
            println!("serve it with: python3 -m http.server --directory {} 8000", destination.display());
            println!("then open: http://localhost:8000/");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("web game export failed: {error}");
            ExitCode::FAILURE
        }
    }
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
        Export::Renderer => {
            build_command.args(["--features", "renderer-proof"]);
            build_command.arg("--target-dir").arg(web_renderer_target_dir(&root));
            (
                web_renderer_build_dir(&root),
                web_renderer_stage_dir(&root),
                "web renderer proof wasm build",
            )
        }
        Export::DemoCheckpoint => {
            build_command.args(["--features", "demo-checkpoint"]);
            build_command.arg("--target-dir").arg(web_demo_target_dir(&root));
            build_command.env(LUA_WASM_CXXFLAGS, PLAYABLE_CXXFLAG);
            (
                web_demo_build_dir(&root),
                web_demo_stage_dir(&root),
                "web Demo checkpoint wasm build",
            )
        }
        Export::Player => {
            build_command.args(["--release", "--features", "player"]);
            build_command.arg("--target-dir").arg(web_player_target_dir(&root));
            build_command.env(LUA_WASM_CXXFLAGS, PLAYABLE_CXXFLAG);
            (
                web_player_build_dir(&root),
                web_player_stage_dir(&root),
                "reusable Bishop Player release wasm build",
            )
        }
    };
    if !run_checked(build_command, build_label) {
        return ExitCode::FAILURE;
    }

    if matches!(variant, Export::Game | Export::Renderer | Export::DemoCheckpoint | Export::Player) && stage_dir.exists() {
        if let Err(error) = fs::remove_dir_all(&stage_dir) {
            eprintln!("failed to clear {}: {error}", stage_dir.display());
            return ExitCode::FAILURE;
        }
    }
    if let Err(error) = fs::create_dir_all(&stage_dir) {
        eprintln!("failed to create {}: {error}", stage_dir.display());
        return ExitCode::FAILURE;
    }

    let html_source = match variant {
        Export::Renderer => WEB_RENDERER_INDEX,
        Export::DemoCheckpoint => WEB_DEMO_INDEX,
        Export::Player => WEB_PLAYER_HTML,
        Export::Shell | Export::Game => WEB_SHELL_INDEX,
    };
    let html_destination = if variant == Export::Player { WEB_PLAYER_HTML } else { WEB_SHELL_INDEX };
    let artifacts = [
        (
            root.join("web_shell").join(html_source),
            stage_dir.join(html_destination),
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

    if variant == Export::Player {
        for (source, destination) in [
            (WEB_PLAYER_CONFIG, WEB_PLAYER_CONFIG),
            (WEB_PLAYER_STORE, WEB_PLAYER_STORE),
            (WEB_PLAYER_TEMPLATE, INDEX_TEMPLATE_FILE),
        ] {
            if !stage_file(&root.join("web_shell").join(source), &stage_dir.join(destination), "player runtime file") {
                return ExitCode::FAILURE;
            }
        }
    }

    if matches!(variant, Export::Shell | Export::Game) {
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

    if variant == Export::Renderer {
        let renderer_urls = [
            web_shell::DEMO_RENDERER_SHEET_PATH.to_owned(),
            web_shell::demo_renderer_atlas_path(),
        ];
        for url in renderer_urls {
            let source = match renderer_source(&root, &url) {
                Ok(source) => source,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            if !stage_file(&source, &stage_dir.join(&url), "web renderer resource") {
                return ExitCode::FAILURE;
            }
        }
    }

    if variant == Export::DemoCheckpoint {
        let resource_stage = stage_dir.join(web_shell::DEMO_CHECKPOINT_ASSET_ROOT);
        if let Err(error) = stage_demo_resources(&demo_resources_dir(&root), &resource_stage) {
            eprintln!("failed to stage Demo resources: {error}");
            return ExitCode::FAILURE;
        }
    }

    match variant {
        Export::Shell => println!("web shell staged at {}", stage_dir.display()),
        Export::Game => println!("playable web fixture staged at {}", stage_dir.display()),
        Export::Renderer => println!("Bishop WebGL2 proof staged at {}", stage_dir.display()),
        Export::DemoCheckpoint => println!("Bishop Demo checkpoint staged at {}", stage_dir.display()),
        Export::Player => {
            println!("reusable Bishop Player runtime staged at {}", stage_dir.display());
            println!("export a game with: cargo xtask export-web-game --resources games/Demo/Resources --out /tmp/bishop-game-site");
            return ExitCode::SUCCESS;
        }
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

fn web_player_target_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-player-build")
}

fn web_player_build_dir(root: &Path) -> PathBuf {
    web_player_target_dir(root).join(WEB_WASM_TARGET).join("release")
}

fn web_player_stage_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-player-runtime")
}

fn parse_export_web_game_args(args: &[String]) -> Result<(PathBuf, PathBuf), String> {
    const USAGE: &str = "Usage: cargo xtask export-web-game --resources <game/Resources> --out <new-site-dir>";
    if args.len() != 4 {
        return Err(USAGE.into());
    }
    let (mut resources, mut destination) = (None, None);
    for pair in args.chunks_exact(2) {
        if pair[1].is_empty() {
            return Err(format!("empty path for {}; {USAGE}", pair[0]));
        }
        match pair[0].as_str() {
            "--resources" if resources.is_none() => resources = Some(PathBuf::from(&pair[1])),
            "--out" if destination.is_none() => destination = Some(PathBuf::from(&pair[1])),
            _ => return Err(format!("unknown or duplicate web export option: {}; {USAGE}", pair[0])),
        }
    }
    Ok((resources.ok_or(USAGE)?, destination.ok_or(USAGE)?))
}

fn web_demo_target_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-demo-build")
}

fn web_demo_build_dir(root: &Path) -> PathBuf {
    web_demo_target_dir(root).join(WEB_WASM_TARGET).join("debug")
}

fn web_demo_stage_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-demo-checkpoint")
}

fn web_renderer_target_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-renderer-build")
}

fn web_renderer_build_dir(root: &Path) -> PathBuf {
    web_renderer_target_dir(root).join(WEB_WASM_TARGET).join("debug")
}

fn web_renderer_stage_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-renderer-proof")
}

fn web_shell_stage_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-shell")
}

fn web_game_stage_dir(root: &Path) -> PathBuf {
    root.join("target").join("web-game")
}

fn renderer_source(root: &Path, asset_url: &str) -> Result<PathBuf, String> {
    let relative = asset_url
        .strip_prefix(web_shell::DEMO_RENDERER_ASSET_ROOT)
        .ok_or_else(|| format!("renderer URL outside Demo root: {asset_url}"))?;
    let relative = Path::new(relative);
    if relative.components().next().is_none()
        || !relative.components().all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("renderer URL is not a safe relative path: {asset_url}"));
    }
    Ok(demo_resources_dir(root).join(relative))
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
