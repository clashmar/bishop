mod cache;
mod music;
mod preview;
mod sfx;
#[cfg(test)]
mod test_state;
#[cfg(test)]
mod tests;

#[cfg(feature = "editor")]
use self::preview::{PendingPreview, TrackedPreview, TrackedPreviewSpec};
#[cfg(test)]
use self::test_state::{
    AudioManagerTestState, StartedLoopPlayback, StartedOneShotPlayback,
};
#[cfg(all(test, feature = "editor"))]
use self::test_state::StartedTrackedPreviewPlayback;
use super::command_queue::{self, AudioCommand, PlayMusicRequest};
use super::diagnostics::{self, AudioDiagnosticsSnapshot};
use super::runtime::{self, MusicStopReason, MusicStoppedEvent};
use crate::task::{BackgroundService, FileReadPool};
use bishop::audio::AudioBackend;
use oddio::{Cycle, Frames, FramesSignal, Gain, Handle, Mixer, Speed, Stop};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Handle type for active looping music signals.
type LoopMusicHandle = Handle<Stop<Gain<Cycle<[f32; 2]>>>>;
/// Handle type for active one-shot music signals.
type OneShotMusicHandle = Handle<Stop<Gain<FramesSignal<[f32; 2]>>>>;
/// Handle type for active looping SFX signals.
type LoopHandle = Handle<Stop<Gain<Speed<Cycle<[f32; 2]>>>>>;
#[cfg(feature = "editor")]
type PreviewHandle = Handle<Stop<Gain<Speed<FramesSignal<[f32; 2]>>>>>;

#[derive(Clone)]
struct PendingMusic {
    token: u64,
    request: PlayMusicRequest,
}

enum PendingOneShot {
    Plain,
    Varied { volume: f32, pitch: f32 },
}

struct PendingLoop {
    sound_id: String,
    volume: f32,
    pitch: f32,
}

enum ActiveMusic {
    Looping {
        id: String,
        handle: LoopMusicHandle,
    },
    OneShot {
        id: String,
        handle: OneShotMusicHandle,
        remaining: f32,
    },
}

impl ActiveMusic {
    fn id(&self) -> &str {
        match self {
            Self::Looping { id, .. } | Self::OneShot { id, .. } => id,
        }
    }

    fn stop(&mut self) {
        match self {
            Self::Looping { handle, .. } => {
                handle.control::<Stop<Gain<Cycle<[f32; 2]>>>, _>().stop();
            }
            Self::OneShot { handle, .. } => {
                handle
                    .control::<Stop<Gain<FramesSignal<[f32; 2]>>>, _>()
                    .stop();
            }
        }
    }

    fn set_ratio(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.0, 1.0);
        match self {
            Self::Looping { handle, .. } => {
                handle
                    .control::<Gain<Cycle<[f32; 2]>>, _>()
                    .set_amplitude_ratio(ratio);
            }
            Self::OneShot { handle, .. } => {
                handle
                    .control::<Gain<FramesSignal<[f32; 2]>>, _>()
                    .set_amplitude_ratio(ratio);
            }
        }
    }
}

enum MusicTransition {
    FadeOut {
        remaining: f32,
        duration: f32,
        start_ratio: f32,
        next_music: Option<PlayMusicRequest>,
    },
    Gap {
        remaining: f32,
        next_music: PlayMusicRequest,
    },
    FadeIn {
        remaining: f32,
        duration: f32,
    },
}

/// Manages audio playback. Implements [`BackgroundService`]; call `poll(dt)` once
/// per frame to drain commands and advance fades.
///
/// Build the signal graph with [`AudioManager::new`], then issue [`AudioCommand`]s
/// via [`super::push_audio_command`] to control playback from anywhere in the game.
pub struct AudioManager {
    /// Keeps the audio backend stream alive. Dropping this stops all audio.
    _keep_alive: Box<dyn Send + 'static>,
    /// Music sub-mixer with gain control.
    music_group: Handle<Stop<Gain<Mixer<[f32; 2]>>>>,
    /// SFX sub-mixer with gain control.
    sfx_group: Handle<Stop<Gain<Mixer<[f32; 2]>>>>,
    /// Active music track. `None` when nothing is playing.
    active_music: Option<ActiveMusic>,
    /// Current music transition stage, if any.
    active_transition: Option<MusicTransition>,
    /// Music request waiting for a background decode to complete.
    pending_music: Option<PendingMusic>,
    /// Monotonic token for pending music requests.
    next_music_token: u64,
    /// Current music gain ratio before master/music volume are applied.
    music_ratio: f32,
    /// Decoded audio cache, keyed by sound ID.
    sound_cache: HashMap<String, Arc<Frames<[f32; 2]>>>,
    /// In-flight file reads, keyed by sound ID.
    pending_loads: HashMap<String, PathBuf>,
    /// Shared bounded pool for audio file reads.
    file_read_pool: FileReadPool,
    /// Pending one-shot playback requests waiting on a cold load.
    pending_one_shots: HashMap<String, Vec<PendingOneShot>>,
    /// Pending loop playback requests waiting on a cold load.
    pending_loops: HashMap<u64, PendingLoop>,
    #[cfg(test)]
    test_state: AudioManagerTestState,
    /// Reference counts tracking how many `AudioSource` components reference each sound ID.
    ref_counts: HashMap<String, usize>,
    /// Sound IDs loaded via `preload()` from Lua; pinned sounds are never auto-evicted.
    pinned: HashSet<String>,
    /// Active looping sound handles, keyed by a caller-supplied u64 handle ID.
    active_loops: HashMap<u64, LoopHandle>,
    #[cfg(feature = "editor")]
    /// Pending editor preview requests waiting on a background load.
    pending_previews: HashMap<u64, PendingPreview>,
    #[cfg(feature = "editor")]
    /// Active editor preview handles, keyed by a caller-supplied preview ID.
    tracked_previews: HashMap<u64, TrackedPreview>,
    #[cfg(feature = "editor")]
    preview_time: f32,
    master_volume: f32,
    music_volume: f32,
    sfx_volume: f32,
}

impl AudioManager {
    /// Constructs an `AudioManager` and starts the audio output stream via `B`.
    ///
    /// Builds the signal graph: root mixer → music group (Gain<Mixer>) +
    /// sfx group (Gain<Mixer>). The rendered signal is driven by the backend.
    pub fn new<B: AudioBackend>() -> Self {
        let (mut root_handle, root_signal) = oddio::split(Mixer::<[f32; 2]>::new());

        let music_group_handle = root_handle
            .control::<Mixer<[f32; 2]>, _>()
            .play(Gain::new(Mixer::<[f32; 2]>::new()));

        let sfx_group_handle = root_handle
            .control::<Mixer<[f32; 2]>, _>()
            .play(Gain::new(Mixer::<[f32; 2]>::new()));

        const SAMPLE_RATE: u32 = 44_100;
        let keep_alive: Box<dyn Send + 'static> =
            Box::new(B::start(move |frames: &mut [[f32; 2]]| {
                oddio::run(&root_signal, SAMPLE_RATE, frames);
            }));
        // `root_signal` is owned by the backend render closure, so only the subgroup
        // handles need to be retained after graph construction.

        runtime::set_music_playing(false);

        Self {
            _keep_alive: keep_alive,
            music_group: music_group_handle,
            sfx_group: sfx_group_handle,
            active_music: None,
            active_transition: None,
            pending_music: None,
            next_music_token: 1,
            music_ratio: 1.0,
            sound_cache: HashMap::new(),
            pending_loads: HashMap::new(),
            file_read_pool: FileReadPool::new(),
            pending_one_shots: HashMap::new(),
            pending_loops: HashMap::new(),
            #[cfg(test)]
            test_state: AudioManagerTestState::default(),
            ref_counts: HashMap::new(),
            pinned: HashSet::new(),
            active_loops: HashMap::new(),
            #[cfg(feature = "editor")]
            pending_previews: HashMap::new(),
            #[cfg(feature = "editor")]
            tracked_previews: HashMap::new(),
            #[cfg(feature = "editor")]
            preview_time: 0.0,
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
        }
    }

    /// Returns a snapshot of cached, pinned, and referenced audio IDs.
    pub fn diagnostics_snapshot(&self) -> AudioDiagnosticsSnapshot {
        let loading = self.pending_loads.keys().cloned().collect::<HashSet<_>>();
        diagnostics::snapshot_from_state(
            &self.sound_cache,
            &self.ref_counts,
            &self.pinned,
            &loading,
        )
    }

    fn set_music_ratio(&mut self, ratio: f32) {
        self.music_ratio = ratio.clamp(0.0, 1.0);
        if let Some(active_music) = self.active_music.as_mut() {
            active_music.set_ratio(self.music_ratio);
        }
    }

    /// Updates the combined master × music gain on the music group.
    fn apply_music_gain(&mut self) {
        let linear = self.master_volume * self.music_volume;
        self.music_group
            .control::<Gain<Mixer<[f32; 2]>>, _>()
            .set_amplitude_ratio(linear);
    }

    /// Updates the combined master × sfx gain on the sfx group.
    fn apply_sfx_gain(&mut self) {
        let linear = self.master_volume * self.sfx_volume;
        self.sfx_group
            .control::<Gain<Mixer<[f32; 2]>>, _>()
            .set_amplitude_ratio(linear);
    }

    pub(super) fn clear_pending_requests_for_sound(&mut self, id: &str) {
        let _ = self.pending_one_shots.remove(id);
        self.pending_loops
            .retain(|_, pending| pending.sound_id != id);
        #[cfg(feature = "editor")]
        self.pending_previews
            .retain(|_, pending| pending.sound_id != id);
    }

    fn dispatch_command(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::PlayMusic(request) => self.play_music(request),
            AudioCommand::StopMusic => self.stop_music(),
            AudioCommand::FadeMusic(duration) => self.fade_music(duration),
            AudioCommand::PlaySfx(id) => self.play_sfx(&id),
            AudioCommand::Preload(id) => self.preload(&id),
            AudioCommand::SetMasterVolume(v) => {
                self.master_volume = v.clamp(0.0, 1.0);
                self.apply_music_gain();
                self.apply_sfx_gain();
            }
            AudioCommand::SetMusicVolume(v) => {
                self.music_volume = v.clamp(0.0, 1.0);
                self.apply_music_gain();
            }
            AudioCommand::SetSfxVolume(v) => {
                self.sfx_volume = v.clamp(0.0, 1.0);
                self.apply_sfx_gain();
            }
            AudioCommand::IncrementRefs(ids) => self.increment_refs(&ids),
            AudioCommand::DecrementRefs(ids) => self.decrement_refs(&ids),
            AudioCommand::Unload(id) => {
                self.pinned.remove(&id);
                if !self.ref_counts.contains_key(&id) {
                    self.evict(&id);
                }
            }
            AudioCommand::PlayVariedSfx {
                sounds,
                volume,
                pitch_variation,
                volume_variation,
            } => self.play_varied_sfx(&sounds, volume, pitch_variation, volume_variation),
            #[cfg(feature = "editor")]
            AudioCommand::PlayTrackedPreview {
                handle,
                sounds,
                volume,
                pitch_variation,
                volume_variation,
                looping,
                timeout,
            } => {
                let preview = TrackedPreviewSpec {
                    sounds: &sounds,
                    volume,
                    pitch_variation,
                    volume_variation,
                    looping,
                    timeout,
                };
                self.play_tracked_preview(handle, preview);
            }
            AudioCommand::PlayLoop {
                handle,
                sounds,
                volume,
                pitch_variation,
                volume_variation,
            } => self.play_loop(handle, &sounds, volume, pitch_variation, volume_variation),
            #[cfg(feature = "editor")]
            AudioCommand::StopTrackedPreview(handle) => self.stop_tracked_preview(handle),
            AudioCommand::StopLoop(handle) => self.stop_loop(handle),
        }
    }
}

impl BackgroundService for AudioManager {
    /// Drains the audio command queue and advances any active fade. Must not block.
    fn poll(&mut self, dt: f32) {
        #[cfg(feature = "editor")]
        {
            self.preview_time += dt;
            self.cleanup_tracked_previews();
        }

        self.poll_pending_loads();
        self.tick_playback_state(dt);

        for command in command_queue::drain_audio_commands() {
            self.dispatch_command(command);
        }

        self.resolve_pending_sfx();
        #[cfg(feature = "editor")]
        self.resolve_pending_tracked_previews();
        self.resolve_pending_music();
        self.publish_runtime_state();
    }
}
