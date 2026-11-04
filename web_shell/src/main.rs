#[cfg(any(test, all(target_os = "emscripten", not(any(feature = "demo-checkpoint", feature = "player")))))]
mod audio_probe;
#[cfg(not(any(feature = "renderer-proof", feature = "demo-checkpoint", feature = "player")))]
mod browser_asset;
#[cfg(all(feature = "demo-checkpoint", not(feature = "player"), target_os = "emscripten"))]
mod browser_demo;
#[cfg(all(feature = "player", target_os = "emscripten"))]
mod browser_player;
#[cfg(target_os = "emscripten")]
mod browser_fetch;
#[cfg(any(test, target_os = "emscripten"))]
mod browser_input;
#[cfg(target_os = "emscripten")]
mod browser_loop;
#[cfg(not(any(feature = "renderer-proof", feature = "demo-checkpoint", feature = "player")))]
mod browser_render;
#[cfg(all(feature = "renderer-proof", target_os = "emscripten"))]
mod renderer_proof;
#[cfg(target_os = "emscripten")]
mod emscripten;
#[cfg(all(feature = "player", any(test, target_os = "emscripten")))]
mod player_bundle;
#[cfg(all(feature = "playable-fixture", any(test, target_os = "emscripten")))]
mod fixture_game;
#[cfg(all(feature = "playable-fixture", any(test, target_os = "emscripten")))]
mod fixture_loader;
#[cfg(all(feature = "playable-fixture", target_os = "emscripten"))]
mod fixture_render;

const SHELL_LOADED: &str = "Bishop web shell loaded";

fn main() {
    println!("{SHELL_LOADED}");
    #[cfg(not(any(feature = "renderer-proof", feature = "demo-checkpoint", feature = "player")))]
    browser_render::run_browser_render_probe();
    #[cfg(not(any(feature = "renderer-proof", feature = "demo-checkpoint", feature = "player")))]
    browser_asset::start_browser_asset_probe();
    #[cfg(all(feature = "demo-checkpoint", not(feature = "player"), target_os = "emscripten"))]
    browser_demo::start();
    #[cfg(all(feature = "player", target_os = "emscripten"))]
    browser_player::start();
    #[cfg(all(feature = "renderer-proof", target_os = "emscripten"))]
    renderer_proof::start();
    #[cfg(all(feature = "playable-fixture", target_os = "emscripten"))]
    fixture_loader::start();
    #[cfg(target_os = "emscripten")]
    browser_loop::start();
    #[cfg(not(target_os = "emscripten"))]
    println!("Bishop browser input/frame loop skipped outside Emscripten");
}
