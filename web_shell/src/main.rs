mod browser_render;

const SHELL_LOADED: &str = "Bishop web shell loaded";

fn main() {
    println!("{SHELL_LOADED}");
    browser_render::run_browser_render_probe();
}
