#[cfg(feature = "audio-cpal")]
mod cpal_backend;
#[cfg(all(feature = "audio", not(feature = "audio-cpal"), not(target_os = "emscripten")))]
mod silent_backend;
#[cfg(all(feature = "audio", not(feature = "audio-cpal"), target_os = "emscripten"))]
mod web_audio_backend;

#[cfg(feature = "audio-cpal")]
pub use cpal_backend::CpalBackend;
#[cfg(all(feature = "audio", not(feature = "audio-cpal"), not(target_os = "emscripten")))]
pub use silent_backend::SilentAudioBackend;
#[cfg(all(feature = "audio", not(feature = "audio-cpal"), target_os = "emscripten"))]
pub use web_audio_backend::WebAudioBackend;

/// Audio mixer rate shared by native and browser output.
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;

/// Platform audio backend. Starts the audio output stream and calls `render_fn`
/// each buffer to fill samples. Implementors live in bishop; engine_core never
/// depends on the concrete type.
pub trait AudioBackend: Send + 'static {
    /// Starts audio output. `render_fn` is called on the audio thread each buffer.
    /// The render function receives a mutable slice of stereo frames `[[f32; 2]]`.
    fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(render_fn: F) -> Self
    where
        Self: Sized;
}

/// The audio backend selected for this build's platform and features.
#[cfg(feature = "audio-cpal")]
pub type PlatformAudioBackend = CpalBackend;

/// Browser output through Web Audio.
#[cfg(all(feature = "audio", not(feature = "audio-cpal"), target_os = "emscripten"))]
pub type PlatformAudioBackend = WebAudioBackend;

/// Silent output for non-Emscripten builds without CPAL.
#[cfg(all(feature = "audio", not(feature = "audio-cpal"), not(target_os = "emscripten")))]
pub type PlatformAudioBackend = SilentAudioBackend;
