use glow::HasContext;

use super::clip::ClipRegion;
use super::gl_context::WebContext;
use crate::input::{Input, KeyCode, MouseButton};
use crate::platform::PlatformControl;
use crate::time::Time;
use crate::types::Rect;
use crate::window::{CursorIcon, Window};

impl Input for WebContext {
    fn is_key_down(&self, key: KeyCode) -> bool {
        self.input.is_key_down(key)
    }

    fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.input.is_key_pressed(key)
    }

    fn is_key_released(&self, key: KeyCode) -> bool {
        self.input.is_key_released(key)
    }

    fn any_key_pressed(&self) -> bool {
        self.input.any_key_pressed()
    }

    fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.input.is_mouse_button_down(button)
    }

    fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.input.is_mouse_button_pressed(button)
    }

    fn is_mouse_button_released(&self, button: MouseButton) -> bool {
        self.input.is_mouse_button_released(button)
    }

    fn is_mouse_button_double_clicked(&self, button: MouseButton) -> bool {
        self.input.is_mouse_button_double_clicked(button)
    }

    fn mouse_position(&self) -> (f32, f32) {
        self.input.mouse_position()
    }

    fn mouse_delta_position(&self) -> (f32, f32) {
        self.input.mouse_delta_position()
    }

    fn mouse_wheel(&self) -> (f32, f32) {
        self.input.mouse_wheel()
    }

    fn chars_pressed(&self) -> Vec<char> {
        self.input.chars_pressed()
    }

    fn get_time(&self) -> f64 {
        self.clock.elapsed()
    }

    fn logical_clip_rect(&self) -> Option<Rect> {
        match self.renderer.clips.current() {
            ClipRegion::Unclipped => None,
            ClipRegion::Empty => Some(Rect::new(0.0, 0.0, 0.0, 0.0)),
            ClipRegion::Rect(rect) => Some(rect),
        }
    }
}

impl WebContext {
    fn active_dimensions(&self) -> (i32, i32) {
        if self.renderer.active_target.is_some() {
            return self.renderer.size();
        }
        match self.owner.canvas_size() {
            Ok(dimensions) => dimensions,
            Err(error) => {
                self.owner.record_error(error);
                self.renderer.canvas_size
            }
        }
    }
}

impl Window for WebContext {
    fn screen_width(&self) -> f32 {
        self.active_dimensions().0 as f32
    }

    fn screen_height(&self) -> f32 {
        self.active_dimensions().1 as f32
    }

    fn set_cursor_icon(&mut self, icon: CursorIcon) {
        let css = match icon {
            CursorIcon::Default => "default",
            CursorIcon::Pointer => "pointer",
            CursorIcon::Crosshair => "crosshair",
            CursorIcon::Move => "move",
            CursorIcon::Text => "text",
            CursorIcon::NotAllowed => "not-allowed",
            CursorIcon::EWResize => "ew-resize",
            CursorIcon::NSResize => "ns-resize",
            CursorIcon::NESWResize => "nesw-resize",
            CursorIcon::NWSEResize => "nwse-resize",
        };
        if let Err(error) = self.owner.set_canvas_cursor(css) {
            self.owner.record_error(error);
        }
    }

    fn toggle_fullscreen(&mut self) -> bool {
        match self.owner.toggle_fullscreen() {
            Ok(fullscreen) => fullscreen,
            Err(error) => {
                self.owner.record_error(error);
                false
            }
        }
    }

    fn is_fullscreen(&self) -> bool {
        match self.owner.fullscreen_status() {
            Ok(fullscreen) => fullscreen,
            Err(error) => {
                self.owner.record_error(error);
                false
            }
        }
    }

    fn scale_factor(&self) -> f32 {
        match self.owner.canvas_scale() {
            Ok(scale) => scale,
            Err(error) => {
                self.owner.record_error(error);
                1.0
            }
        }
    }
}

impl Time for WebContext {
    fn get_frame_time(&self) -> f32 {
        self.clock.frame_time()
    }

    fn get_frame_spike_ms(&self) -> f32 {
        self.clock.frame_spike_ms()
    }

    fn update(&mut self) {
        self.input.end_frame();
    }
}

impl PlatformControl for WebContext {
    fn flush_pending_draws(&mut self) {
        if let Err(error) = self.owner.make_current() {
            self.owner.record_error(error);
            return;
        }
        unsafe {
            self.owner.gl.flush();
            let error = self.owner.gl.get_error();
            if error != glow::NO_ERROR {
                self.owner.record_error(format!("WebGL2 draw flush failed: GL error {error}"));
            }
        }
    }

    fn set_close_requested(&mut self, requested: bool) {
        self.close.set_close_requested(requested);
    }

    fn set_exit_confirmed(&mut self, confirmed: bool) {
        self.close.set_exit_confirmed(confirmed);
    }
}
