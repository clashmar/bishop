use bishop::audio::AudioBackend;
use bishop::prelude::{Texture2D, TextureLoader};
use std::cell::Cell;

/// No-op audio backend for tests that construct an [`AudioManager`] without real audio output.
pub(crate) struct TestBackend;

impl AudioBackend for TestBackend {
    fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(_render_fn: F) -> Self
    where
        Self: Sized,
    {
        Self
    }
}

/// Texture loader that always fails but counts how many times each load method was called.
pub(crate) struct CountingFailingLoader {
    /// Number of `load_texture_from_bytes` calls.
    pub(crate) bytes_load_calls: Cell<usize>,
    /// Number of `load_texture_from_path` calls.
    pub(crate) load_calls: Cell<usize>,
}

impl CountingFailingLoader {
    /// Returns a new loader with zeroed call counters.
    pub(crate) fn new() -> Self {
        Self {
            bytes_load_calls: Cell::new(0),
            load_calls: Cell::new(0),
        }
    }
}

impl TextureLoader for CountingFailingLoader {
    fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
        self.bytes_load_calls
            .set(self.bytes_load_calls.get().saturating_add(1));
        Err("expected test byte load failure".to_string())
    }

    fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
        self.load_calls.set(self.load_calls.get() + 1);
        Err("expected test load failure".to_string())
    }

    fn empty_texture(&self) -> Texture2D {
        panic!("empty_texture is not used in asset manager tests")
    }
}
