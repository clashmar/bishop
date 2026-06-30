use crate::ecs::{Entity, SoundGroupId};
use crate::worlds::{RoomId, WorldId};
use std::cell::RefCell;

/// Parameters for starting a music track.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayMusicRequest {
    /// Path relative to `Resources/audio/` without extension.
    pub id: String,
    /// Whether the track should loop until explicitly stopped.
    pub looping: bool,
    /// Fade out the current track over this many seconds before starting.
    pub fade_out: f32,
    /// Wait silently this many seconds before starting the requested track.
    pub gap: f32,
    /// Fade the requested track in over this many seconds after it starts.
    pub fade_in: f32,
}

/// Identifies the gameplay owner for a looping audio playback.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AudioPlaybackOwner {
    /// A loop owned by an entity.
    Entity(Entity),
    /// A loop owned by a room singleton.
    Room(RoomId),
    /// A loop owned by a world singleton.
    World(WorldId),
}

/// Identifies a looping audio playback by owner and authored sound group.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AudioLoopKey {
    /// The gameplay scope that owns this loop.
    pub owner: AudioPlaybackOwner,
    /// The authored sound group being played for that owner.
    pub group: SoundGroupId,
}

impl AudioLoopKey {
    /// Creates a loop key from an owner and authored group.
    pub fn new(owner: AudioPlaybackOwner, group: SoundGroupId) -> Self {
        Self { owner, group }
    }
}

/// Commands that Lua scripts can issue to the audio system.
/// Queued on the main thread, drained by `AudioManager::poll` each frame.
pub enum AudioCommand {
    PlayMusic(PlayMusicRequest),
    StopMusic,
    FadeMusic(f32),
    PlaySfx(String),
    Preload(String),
    SetMasterVolume(f32),
    SetMusicVolume(f32),
    SetSfxVolume(f32),
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
    /// Start a looping sound tracked by owner and group key. If a loop already exists for the key, it is stopped first.
    PlayLoop {
        key: AudioLoopKey,
        sounds: Vec<String>,
        volume: f32,
        pitch_variation: f32,
        volume_variation: f32,
    },
    #[cfg(feature = "editor")]
    /// Stop a tracked editor preview by handle.
    StopTrackedPreview(u64),
    /// Stop looping sounds owned by the given gameplay scope.
    StopLoops {
        owner: AudioPlaybackOwner,
        fade_out: Option<f32>,
    },
}

thread_local! {
    static AUDIO_COMMANDS: RefCell<Vec<AudioCommand>> = const { RefCell::new(Vec::new()) };
}

/// Push a command onto the per-frame audio queue. Safe to call from Lua closures.
pub fn push_audio_command(cmd: AudioCommand) {
    AUDIO_COMMANDS.with(|q| q.borrow_mut().push(cmd));
}

/// Drain all queued commands. Called once per frame by `AudioManager::poll`.
#[cfg(any(test, feature = "test-utils"))]
pub fn drain_audio_commands() -> Vec<AudioCommand> {
    AUDIO_COMMANDS.with(|q| {
        let mut v = q.borrow_mut();
        std::mem::take(&mut *v)
    })
}

/// Drain all queued commands. Called once per frame by `AudioManager::poll`.
#[cfg(not(any(test, feature = "test-utils")))]
pub(crate) fn drain_audio_commands() -> Vec<AudioCommand> {
    AUDIO_COMMANDS.with(|q| {
        let mut v = q.borrow_mut();
        std::mem::take(&mut *v)
    })
}
