use std::cell::RefCell;

/// Why a music track stopped playing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MusicStopReason {
    /// The track reached the end of its samples naturally.
    Completed,
    /// The track was stopped explicitly.
    Stopped,
    /// The track faded out without a replacement.
    Faded,
    /// The track was replaced by another music request.
    Replaced,
}

impl MusicStopReason {
    /// Returns the Lua-facing event string for this reason.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Stopped => "stopped",
            Self::Faded => "faded",
            Self::Replaced => "replaced",
        }
    }
}

/// Event payload for a music stop notification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicStoppedEvent {
    /// The track that stopped.
    pub id: String,
    /// Why playback ended.
    pub reason: MusicStopReason,
    /// The replacement track, when this stop directly transitions into another track.
    pub next_id: Option<String>,
}

#[derive(Default)]
struct AudioRuntimeState {
    music_playing: bool,
    events: Vec<MusicStoppedEvent>,
}

thread_local! {
    static AUDIO_RUNTIME_STATE: RefCell<AudioRuntimeState> = RefCell::new(AudioRuntimeState::default());
}

/// Returns whether any music is currently active.
pub fn is_music_playing() -> bool {
    AUDIO_RUNTIME_STATE.with(|state| state.borrow().music_playing)
}

/// Publishes the current music-active state.
pub fn set_music_playing(is_playing: bool) {
    AUDIO_RUNTIME_STATE.with(|state| {
        state.borrow_mut().music_playing = is_playing;
    });
}

/// Queues a music stop event for the game loop to emit into Lua.
pub fn push_music_stopped_event(event: MusicStoppedEvent) {
    AUDIO_RUNTIME_STATE.with(|state| {
        state.borrow_mut().events.push(event);
    });
}

/// Drains all queued music stop events.
pub fn drain_audio_events() -> Vec<MusicStoppedEvent> {
    AUDIO_RUNTIME_STATE.with(|state| {
        let mut state = state.borrow_mut();
        std::mem::take(&mut state.events)
    })
}

#[cfg(test)]
pub(crate) fn reset_for_tests() {
    AUDIO_RUNTIME_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.music_playing = false;
        state.events.clear();
    });
}
