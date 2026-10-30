#[cfg(any(test, all(target_os = "emscripten", not(feature = "demo-checkpoint"))))]
mod audio_probe;
#[cfg(not(any(feature = "renderer-proof", feature = "demo-checkpoint")))]
mod browser_asset;
#[cfg(all(feature = "demo-checkpoint", target_os = "emscripten"))]
mod browser_demo;
#[cfg(target_os = "emscripten")]
mod browser_fetch;
#[cfg(any(test, target_os = "emscripten"))]
mod browser_input;
#[cfg(target_os = "emscripten")]
mod browser_loop;
#[cfg(not(any(feature = "renderer-proof", feature = "demo-checkpoint")))]
mod browser_render;
#[cfg(all(feature = "renderer-proof", target_os = "emscripten"))]
mod renderer_proof;
#[cfg(target_os = "emscripten")]
mod emscripten;
#[cfg(all(feature = "playable-fixture", any(test, target_os = "emscripten")))]
mod fixture_game;
#[cfg(all(feature = "playable-fixture", any(test, target_os = "emscripten")))]
mod fixture_loader;
#[cfg(all(feature = "playable-fixture", target_os = "emscripten"))]
mod fixture_render;

const SHELL_LOADED: &str = "Bishop web shell loaded";

fn main() {
    println!("{SHELL_LOADED}");
    #[cfg(not(any(feature = "renderer-proof", feature = "demo-checkpoint")))]
    browser_render::run_browser_render_probe();
    #[cfg(not(any(feature = "renderer-proof", feature = "demo-checkpoint")))]
    browser_asset::start_browser_asset_probe();
    #[cfg(all(feature = "demo-checkpoint", target_os = "emscripten"))]
    browser_demo::start();
    #[cfg(all(feature = "renderer-proof", target_os = "emscripten"))]
    renderer_proof::start();
    #[cfg(all(feature = "playable-fixture", target_os = "emscripten"))]
    fixture_loader::start();
    #[cfg(target_os = "emscripten")]
    browser_loop::start();
    #[cfg(not(target_os = "emscripten"))]
    println!("Bishop browser input/frame loop skipped outside Emscripten");
}
