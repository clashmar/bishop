fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten") {
        println!("cargo:rustc-link-arg=-sEXPORTED_FUNCTIONS=_main,_bishop_focus_changed,_bishop_key_down,_bishop_key_up,_bishop_mouse_down,_bishop_mouse_up,_bishop_mouse_move,_bishop_wheel");
        println!("cargo:rustc-link-arg=-sEXPORTED_RUNTIME_METHODS=ccall");
    }
}
