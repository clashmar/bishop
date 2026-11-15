use crate::audio::AudioBackend;

/// Silent backend for builds without native or browser audio output.
pub struct SilentAudioBackend;

impl AudioBackend for SilentAudioBackend {
    fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(_render_fn: F) -> Self {
        Self
    }
}
