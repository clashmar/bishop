#[cfg(test)]
use crate::app::Editor;
#[cfg(test)]
use bishop::audio::AudioBackend;
#[cfg(not(test))]
use bishop::prelude::PlatformAudioBackend;
use engine_core::prelude::AudioManager;
#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static TEST_AUDIO_BACKEND_STARTS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
struct TestAudioBackend;

#[cfg(test)]
impl AudioBackend for TestAudioBackend {
    fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(_render_fn: F) -> Self
    where
        Self: Sized,
    {
        TEST_AUDIO_BACKEND_STARTS.with(|starts| starts.set(starts.get() + 1));
        Self
    }
}

#[cfg(test)]
pub(super) fn default_audio_manager() -> AudioManager {
    AudioManager::new::<TestAudioBackend>()
}

#[cfg(not(test))]
pub(super) fn default_audio_manager() -> AudioManager {
    AudioManager::new::<PlatformAudioBackend>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_default_uses_test_audio_backend() {
        TEST_AUDIO_BACKEND_STARTS.with(|starts| starts.set(0));

        let _editor = Editor::default();

        TEST_AUDIO_BACKEND_STARTS.with(|starts| assert_eq!(starts.get(), 1));
    }
}
