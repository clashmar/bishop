use crate::audio::AudioBackend;

/// Browser-oriented audio backend for WASM runtime builds.
/// Playback stays silent until the browser-audio chunk wires Web Audio.
pub struct WasmAudioBackend;

impl AudioBackend for WasmAudioBackend {
    fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(_render_fn: F) -> Self {
        Self
    }
}
