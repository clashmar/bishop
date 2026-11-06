use std::ffi::CString;
use std::os::raw::{c_char, c_int};

pub(crate) fn run_script_int(script: &str) -> i32 {
    unsafe extern "C" {
        fn emscripten_run_script_int(script: *const c_char) -> c_int;
    }

    let script = CString::new(script).expect("browser script must not contain nul bytes");
    unsafe { emscripten_run_script_int(script.as_ptr()) as i32 }
}
