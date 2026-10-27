fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten") {
        println!("cargo:rustc-link-arg=-sEXPORTED_FUNCTIONS=_main,_bishop_focus_changed,_bishop_key_down,_bishop_key_up,_bishop_mouse_down,_bishop_mouse_up,_bishop_mouse_move,_bishop_wheel,_bishop_audio_fill");
        println!("cargo:rustc-link-arg=-sEXPORTED_RUNTIME_METHODS=ccall,HEAPF32");
        if std::env::var_os("CARGO_FEATURE_RENDERER_PROOF").is_some() {
            println!("cargo:rustc-link-arg=-sMIN_WEBGL_VERSION=2");
            println!("cargo:rustc-link-arg=-sMAX_WEBGL_VERSION=2");
            println!("cargo:rustc-link-arg=-sFULL_ES3=1");
        }
        if std::env::var_os("CARGO_FEATURE_PLAYABLE_FIXTURE").is_some() {
            println!("cargo:rustc-link-arg=-sDEFAULT_TO_CXX=1");
        }
    }
}
