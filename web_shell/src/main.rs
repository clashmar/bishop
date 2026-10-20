mod browser_asset;
mod browser_render;
#[cfg(target_os = "emscripten")]
mod emscripten;

const SHELL_LOADED: &str = "Bishop web shell loaded";

fn main() {
    println!("{SHELL_LOADED}");
    browser_render::run_browser_render_probe();
    browser_asset::start_browser_asset_probe();
}
