#[cfg(feature = "audio-cpal")]
mod cpal_backend;

#[cfg(feature = "audio-cpal")]
pub use cpal_backend::CpalBackend;

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

/// The default audio backend for the current platform, selected by feature flag.
/// Use this in EngineBuilder to avoid hard-coding a backend.
#[cfg(feature = "audio-cpal")]
pub type PlatformAudioBackend = CpalBackend;
