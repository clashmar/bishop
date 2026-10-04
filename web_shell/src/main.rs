const SHELL_LOADED: &str = "Bishop web shell loaded";
const RUNTIME_PENDING: &str =
    "Bishop runtime startup pending: browser PlatformContext/render backend not wired yet";

fn main() {
    println!("{SHELL_LOADED}");
    println!("{RUNTIME_PENDING}");
}
