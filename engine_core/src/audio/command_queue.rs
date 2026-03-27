use std::cell::RefCell;

/// Commands that Lua scripts can issue to the audio system.
/// Queued on the main thread, drained by `AudioManager::poll` each frame.
pub enum AudioCommand {
    PlayMusic(String),
    StopMusic,
    FadeMusic(f32),
    PlaySfx(String),
    Preload(String),
    SetMasterVolume(f32),
    SetMusicVolume(f32),
    SetSfxVolume(f32),
    /// Increment reference counts for a batch of sound IDs, loading each if not cached.
    IncrementRefs(Vec<String>),
    /// Decrement reference counts for a batch of sound IDs, evicting unpinned sounds that reach zero.
    DecrementRefs(Vec<String>),
    /// Explicitly unpin and evict a sound from the cache if its reference count is zero.
    Unload(String),
    /// Play a one-shot sound with random selection from the list and optional pitch/volume variation.
    PlayVariedSfx {
        sounds: Vec<String>,
        volume: f32,
        pitch_variation: f32,
        volume_variation: f32,
    },
    #[cfg(feature = "editor")]
    /// Start a tracked editor preview, replacing any existing preview with the same handle.
    PlayTrackedPreview {
        handle: u64,
        sounds: Vec<String>,
        volume: f32,
        pitch_variation: f32,
        volume_variation: f32,
        looping: bool,
        timeout: f32,
    },
    /// Start a looping sound tracked by a u64 handle key. If a loop already exists for the handle, it is stopped first.
    PlayLoop {
        handle: u64,
        sounds: Vec<String>,
        volume: f32,
        pitch_variation: f32,
        volume_variation: f32,
    },
    #[cfg(feature = "editor")]
    /// Stop a tracked editor preview by handle.
    StopTrackedPreview(u64),
    /// Stop a looping sound by its handle key.
    StopLoop(u64),
}

thread_local! {
    static AUDIO_COMMANDS: RefCell<Vec<AudioCommand>> = const { RefCell::new(Vec::new()) };
}

/// Push a command onto the per-frame audio queue. Safe to call from Lua closures.
pub fn push_audio_command(cmd: AudioCommand) {
    AUDIO_COMMANDS.with(|q| q.borrow_mut().push(cmd));
}

/// Drain all queued commands. Called once per frame by `AudioManager::poll`.
pub(crate) fn drain_audio_commands() -> Vec<AudioCommand> {
    AUDIO_COMMANDS.with(|q| {
        let mut v = q.borrow_mut();
        std::mem::take(&mut *v)
    })
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "editor")]
    use super::*;

    #[cfg(feature = "editor")]
    #[test]
    fn tracked_preview_commands_can_be_queued_and_drained() {
        let _ = drain_audio_commands();

        push_audio_command(AudioCommand::PlayTrackedPreview {
            handle: 7,
            sounds: vec!["ui/click".to_string()],
            volume: 0.75,
            pitch_variation: 0.1,
            volume_variation: 0.2,
            looping: true,
            timeout: 1.5,
        });
        push_audio_command(AudioCommand::StopTrackedPreview(7));

        let commands = drain_audio_commands();
        assert_eq!(commands.len(), 2);
        match &commands[0] {
            AudioCommand::PlayTrackedPreview {
                handle,
                sounds,
                volume,
                pitch_variation,
                volume_variation,
                looping,
                timeout,
            } => {
                assert_eq!(*handle, 7);
                assert_eq!(sounds, &vec!["ui/click".to_string()]);
                assert_eq!(*volume, 0.75);
                assert_eq!(*pitch_variation, 0.1);
                assert_eq!(*volume_variation, 0.2);
                assert!(*looping);
                assert_eq!(*timeout, 1.5);
            }
            _ => panic!("expected PlayTrackedPreview"),
        }
        match &commands[1] {
            AudioCommand::StopTrackedPreview(handle) => assert_eq!(*handle, 7),
            _ => panic!("expected StopTrackedPreview"),
        }
    }
}
