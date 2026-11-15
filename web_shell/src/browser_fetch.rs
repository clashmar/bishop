use std::ffi::{CString, c_void};
use std::os::raw::{c_char, c_int};

struct Request {
    url: String,
    done: Box<dyn FnOnce(Result<Vec<u8>, String>)>,
}

unsafe extern "C" {
    fn emscripten_async_wget_data(
        url: *const c_char,
        userdata: *mut c_void,
        onload: extern "C" fn(*mut c_void, *mut c_void, c_int),
        onerror: extern "C" fn(*mut c_void),
    );
}

pub(crate) fn fetch(url: &str, done: impl FnOnce(Result<Vec<u8>, String>) + 'static) {
    let url_cstring = match CString::new(url) {
        Ok(url_cstring) => url_cstring,
        Err(error) => {
            done(Err(format!("invalid HTTP URL {url}: {error}")));
            return;
        }
    };
    let request = Box::new(Request {
        url: url.to_owned(),
        done: Box::new(done),
    });
    unsafe {
        emscripten_async_wget_data(
            url_cstring.as_ptr(),
            Box::into_raw(request).cast(),
            loaded,
            failed,
        );
    }
}

extern "C" fn loaded(userdata: *mut c_void, data: *mut c_void, size: c_int) {
    let Request { url, done } = *unsafe { Box::from_raw(userdata.cast::<Request>()) };
    if data.is_null() || size < 0 {
        done(Err(format!("invalid HTTP data for {url}")));
        return;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size as usize).to_vec() };
    done(Ok(bytes));
}

extern "C" fn failed(userdata: *mut c_void) {
    let Request { url, done } = *unsafe { Box::from_raw(userdata.cast::<Request>()) };
    done(Err(format!("fetch failed for {url}")));
}
