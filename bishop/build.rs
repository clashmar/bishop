fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten")
        && std::env::var_os("CARGO_FEATURE_WEB_GL").is_some()
    {
        println!("cargo:rerun-if-changed=src/web/context_bridge.c");
        cc::Build::new()
            .file("src/web/context_bridge.c")
            .compile("bishop_webgl");
    }
}
