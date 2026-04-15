#[cfg(all(target_os = "windows", not(debug_assertions)))]
/// Windows .exe for the game playtest binary.
pub static PLAYTEST_EXE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/binaries/game-playtest.exe"
));

#[cfg(all(target_os = "macos", not(debug_assertions)))]
/// Mac binary for the game playtest.
pub static PLAYTEST_BIN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/binaries/game-playtest"
));
