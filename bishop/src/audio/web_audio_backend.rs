use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::audio::{AUDIO_SAMPLE_RATE, AudioBackend};

const BUFFER_FRAMES: usize = 2_048;

type RenderCallback = Box<dyn FnMut(&mut [[f32; 2]]) + Send>;

struct AudioState {
    render: RenderCallback,
    frames: Vec<[f32; 2]>,
}

/// Browser Web Audio output retained for the lifetime of its render callback.
pub struct WebAudioBackend {
    state: Option<Box<AudioState>>,
}

unsafe extern "C" {
    fn emscripten_run_script_int(script: *const c_char) -> i32;
}

fn run_audio_script(script: String) -> i32 {
    let script = CString::new(script).expect("numeric audio script cannot contain NUL");
    unsafe { emscripten_run_script_int(script.as_ptr()) }
}

impl AudioBackend for WebAudioBackend {
    fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(render_fn: F) -> Self {
        let state = Box::new(AudioState {
            render: Box::new(render_fn),
            frames: vec![[0.0; 2]; BUFFER_FRAMES],
        });
        let address = (&*state as *const AudioState).cast::<c_void>() as usize;
        let result = run_audio_script(format!(
            "window.bishopAudioStart?.({address}, {AUDIO_SAMPLE_RATE}, {BUFFER_FRAMES}) ?? 0"
        ));
        match result {
            1 => Self { state: Some(state) },
            0 => {
                println!("Bishop Web Audio blocked: browser audio adapter is missing");
                Self { state: None }
            }
            _ => Self { state: None },
        }
    }
}

impl Drop for WebAudioBackend {
    fn drop(&mut self) {
        if let Some(state) = &self.state {
            let address = (&**state as *const AudioState).cast::<c_void>() as usize;
            run_audio_script(format!("window.bishopAudioStop?.({address}) ?? 0"));
        }
    }
}

/// Fills an owned interleaved stereo buffer for the browser audio adapter.
#[unsafe(no_mangle)]
pub extern "C" fn bishop_audio_fill(state: *mut c_void, frame_count: i32) -> *const f32 {
    if state.is_null() || frame_count != BUFFER_FRAMES as i32 {
        return ptr::null();
    }
    // SAFETY: the browser adapter stops callbacks before the backend drops its state
    let state = unsafe { &mut *state.cast::<AudioState>() };
    (state.render)(&mut state.frames);
    state.frames.as_ptr().cast::<f32>()
}
