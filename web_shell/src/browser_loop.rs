use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;

use bishop::input::{KeyCode, KeyboardMouseState, MouseButton};

use crate::audio_probe::AudioProbe;

struct BrowserRuntime {
    input: KeyboardMouseState,
    audio: AudioProbe,
    active: bool,
    last_ms: Option<f64>,
    x: f32,
    y: f32,
    frame: u64,
    last_button: i32,
}

impl BrowserRuntime {
    fn new() -> Self {
        Self {
            input: KeyboardMouseState::new(),
            audio: AudioProbe::new(),
            active: false,
            last_ms: None,
            x: 300.0,
            y: 130.0,
            frame: 0,
            last_button: 0,
        }
    }
}

thread_local! {
    static RUNTIME: RefCell<BrowserRuntime> = RefCell::new(BrowserRuntime::new());
}

pub(crate) fn with_input(f: impl FnOnce(&mut KeyboardMouseState)) {
    RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        if runtime.active {
            f(&mut runtime.input);
        }
    });
}

pub(crate) fn mouse_down(button: MouseButton, x: f32, y: f32, time_ms: f64) {
    RUNTIME.with(|cell| {
        let mut runtime = cell.borrow_mut();
        if !runtime.active {
            return;
        }
        runtime.input.on_mouse_move(x, y);
        runtime.input.on_mouse_down(button, time_ms / 1000.0, (x, y));
        if button == MouseButton::Left {
            runtime.audio.trigger();
        }
    });
}

pub(crate) fn focus_changed(active: bool) {
    RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        if !active {
            runtime.input.clear_on_focus_loss();
            runtime.audio.cancel();
            runtime.last_ms = None;
        } else if !runtime.active {
            runtime.last_ms = None;
        }
        runtime.active = active;
    });
}

unsafe extern "C" {
    fn emscripten_get_now() -> f64;
    fn emscripten_set_main_loop_arg(
        callback: extern "C" fn(*mut c_void),
        arg: *mut c_void,
        fps: i32,
        simulate_infinite_loop: i32,
    );
    fn emscripten_cancel_main_loop();
}

pub(crate) fn start() {
    if crate::emscripten::run_script_int("window.bishopInstallInput?.() ? 1 : 0") != 1 {
        println!("Bishop browser input/frame loop blocked: canvas, 2D context, or event bridge unavailable");
        return;
    }
    println!("Bishop browser input/frame loop started");
    unsafe { emscripten_set_main_loop_arg(frame, ptr::null_mut(), 0, 1) };
}

extern "C" fn frame(_unused: *mut c_void) {
    let now_ms = unsafe { emscripten_get_now() };
    let drawn = RUNTIME.with(|cell| {
        let mut runtime = cell.borrow_mut();
        if !runtime.active {
            return true;
        }

        let dt = runtime
            .last_ms
            .map_or(0.0, |previous| ((now_ms - previous) / 1000.0).clamp(0.0, 0.05))
            as f32;
        runtime.last_ms = Some(now_ms);
        let speed = 140.0 * dt;
        if runtime.input.is_key_down(KeyCode::A) || runtime.input.is_key_down(KeyCode::Left) {
            runtime.x -= speed;
        }
        if runtime.input.is_key_down(KeyCode::D) || runtime.input.is_key_down(KeyCode::Right) {
            runtime.x += speed;
        }
        if runtime.input.is_key_down(KeyCode::W) || runtime.input.is_key_down(KeyCode::Up) {
            runtime.y -= speed;
        }
        if runtime.input.is_key_down(KeyCode::S) || runtime.input.is_key_down(KeyCode::Down) {
            runtime.y += speed;
        }
        runtime.x = runtime.x.clamp(0.0, 620.0);
        runtime.y = runtime.y.clamp(65.0, 275.0);

        if runtime.input.is_mouse_button_pressed(MouseButton::Left) {
            runtime.last_button = 1;
        }
        if runtime.input.is_mouse_button_pressed(MouseButton::Middle) {
            runtime.last_button = 2;
        }
        if runtime.input.is_mouse_button_pressed(MouseButton::Right) {
            runtime.last_button = 3;
        }
        runtime.frame += 1;
        let (mx, my) = runtime.input.mouse_position();
        let drawn = crate::browser_render::render_interactive_frame(
            runtime.x,
            runtime.y,
            mx,
            my,
            runtime.last_button,
            runtime.frame,
        );
        runtime.input.end_frame();
        drawn
    });
    if !drawn {
        println!("Bishop browser input/frame loop blocked: canvas drawing failed");
        unsafe { emscripten_cancel_main_loop() };
    }
}
