mod browser_asset;
#[cfg(any(test, target_os = "emscripten"))]
mod browser_input;
#[cfg(target_os = "emscripten")]
mod browser_loop;
mod browser_render;
#[cfg(target_os = "emscripten")]
mod emscripten;

const SHELL_LOADED: &str = "Bishop web shell loaded";

fn main() {
    println!("{SHELL_LOADED}");
    browser_render::run_browser_render_probe();
    browser_asset::start_browser_asset_probe();
    #[cfg(target_os = "emscripten")]
    browser_loop::start();
    #[cfg(not(target_os = "emscripten"))]
    println!("Bishop browser input/frame loop skipped outside Emscripten");
}
