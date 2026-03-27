pub mod audio_source;
pub mod diagnostics;
pub mod command_queue;
pub mod loader;
pub mod runtime;
#[cfg(test)]
mod tests;

pub use audio_source::{AudioGroup, AudioSource, SoundGroupId};
pub use diagnostics::{AudioDiagnosticsEntry, AudioDiagnosticsSnapshot};
pub use command_queue::{AudioCommand, PlayMusicRequest, push_audio_command};
pub use loader::load_wav;
pub use runtime::{MusicStopReason, MusicStoppedEvent};

use crate::task::BackgroundService;
use bishop::audio::AudioBackend;
use oddio::{Cycle, Frames, FramesSignal, Gain, Handle, Mixer, Speed, Stop};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Handle type for active looping music signals.
type LoopMusicHandle = Handle<Stop<Gain<Cycle<[f32; 2]>>>>;
/// Handle type for active one-shot music signals.
type OneShotMusicHandle = Handle<Stop<Gain<FramesSignal<[f32; 2]>>>>;
/// Handle type for active looping SFX signals.
type LoopHandle = Handle<Stop<Gain<Speed<Cycle<[f32; 2]>>>>>;
#[cfg(feature = "editor")]
type PreviewHandle = Handle<Stop<Gain<Speed<FramesSignal<[f32; 2]>>>>>;

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

#[cfg(feature = "editor")]
struct TrackedPreview {
    handle: PreviewSignal,
    expires_at: f32,
}

#[cfg(feature = "editor")]
enum PreviewSignal {
    OneShot(PreviewHandle),
    Loop(LoopHandle),
}

#[cfg(feature = "editor")]
struct TrackedPreviewSpec<'a> {
    sounds: &'a [String],
    volume: f32,
    pitch_variation: f32,
    volume_variation: f32,
    looping: bool,
    timeout: f32,
}

/// Manages audio playback. Implements [`BackgroundService`]; call `poll(dt)` once
/// per frame to drain commands and advance fades.
///
/// Build the signal graph with [`AudioManager::new`], then issue [`AudioCommand`]s
/// via [`push_audio_command`] to control playback from anywhere in the game.
pub struct AudioManager {
    /// Keeps the audio backend stream alive. Dropping this stops all audio.
    _keep_alive: Box<dyn Send + 'static>,
    /// Root stereo mixer handle — retained to keep the signal graph alive.
    _root: Handle<Mixer<[f32; 2]>>,
    /// Music sub-mixer with gain control.
    music_group: Handle<Stop<Gain<Mixer<[f32; 2]>>>>,
    /// SFX sub-mixer with gain control.
    sfx_group: Handle<Stop<Gain<Mixer<[f32; 2]>>>>,
    /// Active music track. `None` when nothing is playing.
    active_music: Option<ActiveMusic>,
    /// Current music transition stage, if any.
    active_transition: Option<MusicTransition>,
    /// Current music gain ratio before master/music volume are applied.
    music_ratio: f32,
    /// Decoded audio cache, keyed by sound ID.
    sound_cache: HashMap<String, Arc<Frames<[f32; 2]>>>,
    /// Reference counts tracking how many `AudioSource` components reference each sound ID.
    ref_counts: HashMap<String, usize>,
    /// Sound IDs loaded via `preload()` from Lua; pinned sounds are never auto-evicted.
    pinned: HashSet<String>,
    /// Active looping sound handles, keyed by a caller-supplied u64 handle ID.
    active_loops: HashMap<u64, LoopHandle>,
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
        // Root stereo mixer — split gives us a Handle for control and a SplitSignal for rendering.
        let (mut root_handle, root_signal) = oddio::split(Mixer::<[f32; 2]>::new());

        // Music sub-mixer wrapped in gain, played into the root.
        let music_group_handle = root_handle
            .control::<Mixer<[f32; 2]>, _>()
            .play(Gain::new(Mixer::<[f32; 2]>::new()));

        // SFX sub-mixer wrapped in gain, played into the root.
        let sfx_group_handle = root_handle
            .control::<Mixer<[f32; 2]>, _>()
            .play(Gain::new(Mixer::<[f32; 2]>::new()));

        // Start the backend. The render closure captures root_signal and calls
        // oddio::run each buffer callback. Sample rate is 44100 Hz — the frames
        // embedded rate handles resampling internally.
        const SAMPLE_RATE: u32 = 44100;
        let keep_alive: Box<dyn Send + 'static> =
            Box::new(B::start(move |frames: &mut [[f32; 2]]| {
                oddio::run(&root_signal, SAMPLE_RATE, frames);
            }));

        runtime::set_music_playing(false);

        Self {
            _keep_alive: keep_alive,
            _root: root_handle,
            music_group: music_group_handle,
            sfx_group: sfx_group_handle,
            active_music: None,
            active_transition: None,
            music_ratio: 1.0,
            sound_cache: HashMap::new(),
            ref_counts: HashMap::new(),
            pinned: HashSet::new(),
            active_loops: HashMap::new(),
            #[cfg(feature = "editor")]
            tracked_previews: HashMap::new(),
            #[cfg(feature = "editor")]
            preview_time: 0.0,
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
        }
    }

    /// Loads sound `id` from disk if not cached, returning a shared reference.
    fn load_or_cached(&mut self, id: &str) -> Option<Arc<Frames<[f32; 2]>>> {
        if let Some(frames) = self.sound_cache.get(id) {
            return Some(frames.clone());
        }
        match load_wav(id) {
            Ok(frames) => {
                self.sound_cache.insert(id.to_owned(), frames.clone());
                Some(frames)
            }
            Err(e) => {
                log::error!("AudioManager: failed to load '{id}': {e}");
                None
            }
        }
    }

    /// Returns a snapshot of cached, pinned, and referenced audio IDs.
    pub fn diagnostics_snapshot(&self) -> AudioDiagnosticsSnapshot {
        diagnostics::snapshot_from_state(&self.sound_cache, &self.ref_counts, &self.pinned)
    }

    fn set_music_ratio(&mut self, ratio: f32) {
        self.music_ratio = ratio.clamp(0.0, 1.0);
        if let Some(active_music) = self.active_music.as_mut() {
            active_music.set_ratio(self.music_ratio);
        }
    }

    fn begin_fade_out(&mut self, duration: f32, next_music: Option<PlayMusicRequest>) {
        if duration <= 0.0 {
            match next_music {
                Some(request) => self.replace_music_now(request),
                None => self.finish_music(MusicStopReason::Faded, None),
            }
            return;
        }

        self.active_transition = Some(MusicTransition::FadeOut {
            remaining: duration,
            duration,
            start_ratio: self.music_ratio,
            next_music,
        });
    }

    fn queue_music_start(&mut self, request: PlayMusicRequest) {
        let request = PlayMusicRequest {
            fade_out: 0.0,
            gap: request.gap.max(0.0),
            fade_in: request.fade_in.max(0.0),
            ..request
        };

        if request.gap > 0.0 {
            self.active_transition = Some(MusicTransition::Gap {
                remaining: request.gap,
                next_music: PlayMusicRequest {
                    gap: 0.0,
                    ..request
                },
            });
            self.set_music_ratio(1.0);
            return;
        }

        self.start_music(request);
    }

    /// Starts playing a music track according to the supplied request.
    fn start_music(&mut self, request: PlayMusicRequest) {
        let Some(frames) = self.load_or_cached(&request.id) else {
            self.active_transition = None;
            self.set_music_ratio(1.0);
            return;
        };

        let fade_in = request.fade_in.max(0.0);
        let initial_ratio = if fade_in > 0.0 { 0.0 } else { 1.0 };
        self.active_transition = None;
        self.music_ratio = initial_ratio;

        if request.looping {
            let mut signal = Gain::new(Cycle::new(frames));
            signal.set_amplitude_ratio(initial_ratio);
            let track_handle = self
                .music_group
                .control::<Mixer<[f32; 2]>, _>()
                .play(signal);
            self.active_music = Some(ActiveMusic::Looping {
                id: request.id,
                handle: track_handle,
            });
        } else {
            let runtime = frames.runtime() as f32;
            let mut signal = Gain::new(FramesSignal::from(frames));
            signal.set_amplitude_ratio(initial_ratio);
            let track_handle = self
                .music_group
                .control::<Mixer<[f32; 2]>, _>()
                .play(signal);
            self.active_music = Some(ActiveMusic::OneShot {
                id: request.id,
                handle: track_handle,
                remaining: runtime,
            });
        }

        if fade_in > 0.0 {
            self.active_transition = Some(MusicTransition::FadeIn {
                remaining: fade_in,
                duration: fade_in,
            });
        }
    }

    fn replace_music_now(&mut self, request: PlayMusicRequest) {
        if self.active_music.is_some() {
            self.finish_music(MusicStopReason::Replaced, Some(request.id.clone()));
        }
        self.queue_music_start(request);
    }

    /// Begins playing music, optionally after fading out the current track.
    fn play_music(&mut self, request: PlayMusicRequest) {
        let request = PlayMusicRequest {
            fade_out: request.fade_out.max(0.0),
            gap: request.gap.max(0.0),
            fade_in: request.fade_in.max(0.0),
            ..request
        };

        if self.active_music.is_none() {
            self.active_transition = None;
            self.queue_music_start(PlayMusicRequest {
                fade_out: 0.0,
                ..request
            });
            return;
        }

        if request.fade_out > 0.0 {
            self.begin_fade_out(
                request.fade_out,
                Some(PlayMusicRequest {
                    fade_out: 0.0,
                    ..request
                }),
            );
            return;
        }

        self.replace_music_now(request);
    }

    fn finish_music(&mut self, reason: MusicStopReason, next_id: Option<String>) {
        let Some(mut music) = self.active_music.take() else {
            self.active_transition = None;
            self.set_music_ratio(1.0);
            return;
        };

        let id = music.id().to_string();
        music.stop();
        self.active_transition = None;
        self.set_music_ratio(1.0);
        runtime::push_music_stopped_event(MusicStoppedEvent {
            id,
            reason,
            next_id,
        });
    }

    /// Stops the active music track immediately.
    fn stop_music(&mut self) {
        if self.active_music.is_some() {
            self.finish_music(MusicStopReason::Stopped, None);
            return;
        }

        self.active_transition = None;
        self.set_music_ratio(1.0);
    }

    /// Begins a fade-out of the active music over `duration` seconds.
    fn fade_music(&mut self, duration: f32) {
        if self.active_music.is_some() {
            self.begin_fade_out(duration.max(0.0), None);
            return;
        }

        self.active_transition = None;
        self.set_music_ratio(1.0);
    }

    /// Plays a one-shot SFX by ID. Fire and forget.
    fn play_sfx(&mut self, id: &str) {
        let Some(frames) = self.load_or_cached(id) else {
            return;
        };
        let signal = FramesSignal::from(frames);
        self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
    }

    /// Preloads a sound into the cache without playing it and pins it against auto-eviction.
    fn preload(&mut self, id: &str) {
        self.load_or_cached(id);
        self.pinned.insert(id.to_owned());
    }

    /// Evicts a sound from the cache if it is not pinned.
    fn evict(&mut self, id: &str) {
        if !self.pinned.contains(id) {
            self.sound_cache.remove(id);
        }
    }

    /// Increments reference counts for the given IDs, loading each sound if not already cached.
    pub(crate) fn increment_refs(&mut self, ids: &[String]) {
        for id in ids {
            *self.ref_counts.entry(id.to_owned()).or_insert(0) += 1;
            self.load_or_cached(id);
        }
    }

    /// Decrements reference counts for the given IDs. Evicts unpinned sounds whose count reaches zero.
    pub(crate) fn decrement_refs(&mut self, ids: &[String]) {
        for id in ids {
            let reached_zero = if let Some(count) = self.ref_counts.get_mut(id.as_str()) {
                *count = count.saturating_sub(1);
                *count == 0
            } else {
                false
            };
            if reached_zero {
                self.ref_counts.remove(id.as_str());
                self.evict(id);
            }
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

    /// Applies a random variation to `base`, clamped to [0.0, 1.0].
    /// Returns `base` unchanged when `variation` is zero.
    fn apply_variation(base: f32, variation: f32) -> f32 {
        if variation == 0.0 {
            return base;
        }
        let delta = rand::thread_rng().gen_range(-variation..=variation);
        (base + delta).clamp(0.0, 1.0)
    }

    /// Selects a random element from `sounds`, returning `None` when the slice is empty.
    fn pick_sound(sounds: &[String]) -> Option<&str> {
        if sounds.is_empty() {
            return None;
        }
        let idx = rand::thread_rng().gen_range(0..sounds.len());
        Some(&sounds[idx])
    }

    /// Plays a one-shot sound chosen randomly from `sounds`, with optional pitch and volume variation.
    fn play_varied_sfx(
        &mut self,
        sounds: &[String],
        volume: f32,
        pitch_variation: f32,
        volume_variation: f32,
    ) {
        let Some(id) = Self::pick_sound(sounds) else {
            return;
        };
        let Some(frames) = self.load_or_cached(id) else {
            return;
        };
        let final_volume = Self::apply_variation(volume, volume_variation);
        let final_pitch =
            (1.0 + rand::thread_rng().gen_range(-pitch_variation..=pitch_variation)).max(0.1);
        let mut signal = Gain::new(Speed::new(FramesSignal::from(frames)));
        signal.set_amplitude_ratio(final_volume);
        let mut handle = self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
        handle
            .control::<Speed<FramesSignal<[f32; 2]>>, _>()
            .set_speed(final_pitch);
    }

    /// Starts a looping sound for the given `handle_key`, replacing any existing loop for that key.
    fn play_loop(
        &mut self,
        handle_key: u64,
        sounds: &[String],
        volume: f32,
        pitch_variation: f32,
        volume_variation: f32,
    ) {
        self.stop_loop(handle_key);
        let Some(id) = Self::pick_sound(sounds) else {
            return;
        };
        let Some(frames) = self.load_or_cached(id) else {
            return;
        };
        let final_volume = Self::apply_variation(volume, volume_variation);
        let final_pitch =
            (1.0 + rand::thread_rng().gen_range(-pitch_variation..=pitch_variation)).max(0.1);
        let mut signal = Gain::new(Speed::new(Cycle::new(frames)));
        signal.set_amplitude_ratio(final_volume);
        let mut handle = self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
        handle
            .control::<Speed<Cycle<[f32; 2]>>, _>()
            .set_speed(final_pitch);
        self.active_loops.insert(handle_key, handle);
    }

    /// Stops the looping sound associated with `handle_key`, if one exists.
    fn stop_loop(&mut self, handle_key: u64) {
        if let Some(mut handle) = self.active_loops.remove(&handle_key) {
            handle
                .control::<Stop<Gain<Speed<Cycle<[f32; 2]>>>>, _>()
                .stop();
        }
    }

    #[cfg(feature = "editor")]
    fn play_tracked_preview(&mut self, handle_key: u64, spec: TrackedPreviewSpec<'_>) {
        self.stop_tracked_preview(handle_key);
        let Some(id) = Self::pick_sound(spec.sounds) else {
            return;
        };
        let Some(frames) = self.load_or_cached(id) else {
            return;
        };
        let final_volume = Self::apply_variation(spec.volume, spec.volume_variation);
        let final_pitch = (1.0
            + rand::thread_rng().gen_range(-spec.pitch_variation..=spec.pitch_variation))
        .max(0.1);

        let signal = if spec.looping {
            let mut signal = Gain::new(Speed::new(Cycle::new(frames)));
            signal.set_amplitude_ratio(final_volume);
            let mut handle = self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
            handle
                .control::<Speed<Cycle<[f32; 2]>>, _>()
                .set_speed(final_pitch);
            PreviewSignal::Loop(handle)
        } else {
            let mut signal = Gain::new(Speed::new(FramesSignal::from(frames)));
            signal.set_amplitude_ratio(final_volume);
            let mut handle = self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
            handle
                .control::<Speed<FramesSignal<[f32; 2]>>, _>()
                .set_speed(final_pitch);
            PreviewSignal::OneShot(handle)
        };

        self.tracked_previews.insert(
            handle_key,
            TrackedPreview {
                handle: signal,
                expires_at: self.preview_time + spec.timeout.max(0.0),
            },
        );
    }

    #[cfg(feature = "editor")]
    fn stop_tracked_preview(&mut self, handle_key: u64) {
        if let Some(tracked_preview) = self.tracked_previews.remove(&handle_key) {
            match tracked_preview.handle {
                PreviewSignal::OneShot(mut handle) => {
                    handle
                        .control::<Stop<Gain<Speed<FramesSignal<[f32; 2]>>>>, _>()
                        .stop();
                }
                PreviewSignal::Loop(mut handle) => {
                    handle
                        .control::<Stop<Gain<Speed<Cycle<[f32; 2]>>>>, _>()
                        .stop();
                }
            }
        }
    }

    #[cfg(feature = "editor")]
    fn cleanup_tracked_previews(&mut self) {
        let expired = self
            .tracked_previews
            .iter()
            .filter_map(|(handle, preview)| {
                if self.preview_time >= preview.expires_at {
                    Some(*handle)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for handle in expired {
            self.stop_tracked_preview(handle);
        }
    }

    fn tick_music_completion(&mut self, dt: f32) {
        let finished = match self.active_music.as_mut() {
            Some(ActiveMusic::OneShot { remaining, .. }) => {
                *remaining -= dt.max(0.0);
                *remaining <= 0.0
            }
            _ => false,
        };

        if !finished {
            return;
        }

        let replacement = match self.active_transition.take() {
            Some(MusicTransition::FadeOut {
                next_music: Some(request),
                ..
            }) => Some(request),
            _ => None,
        };

        match replacement {
            Some(request) => {
                self.finish_music(MusicStopReason::Replaced, Some(request.id.clone()));
                self.queue_music_start(request);
            }
            None => self.finish_music(MusicStopReason::Completed, None),
        }
    }

    fn tick_transition(&mut self, dt: f32) {
        enum TransitionAction {
            FadeOutComplete {
                next_music: Option<PlayMusicRequest>,
            },
            GapComplete {
                next_music: PlayMusicRequest,
            },
            FadeInComplete,
            UpdateRatio(f32),
        }

        let action = match self.active_transition.as_mut() {
            Some(MusicTransition::FadeOut {
                remaining,
                duration,
                start_ratio,
                next_music,
            }) => {
                *remaining -= dt;
                if *remaining <= 0.0 {
                    TransitionAction::FadeOutComplete {
                        next_music: next_music.clone(),
                    }
                } else {
                    let ratio = (*remaining / *duration).clamp(0.0, 1.0) * *start_ratio;
                    TransitionAction::UpdateRatio(ratio)
                }
            }
            Some(MusicTransition::Gap {
                remaining,
                next_music,
            }) => {
                *remaining -= dt;
                if *remaining <= 0.0 {
                    TransitionAction::GapComplete {
                        next_music: next_music.clone(),
                    }
                } else {
                    return;
                }
            }
            Some(MusicTransition::FadeIn {
                remaining,
                duration,
            }) => {
                *remaining -= dt;
                if *remaining <= 0.0 {
                    TransitionAction::FadeInComplete
                } else {
                    let ratio = 1.0 - (*remaining / *duration).clamp(0.0, 1.0);
                    TransitionAction::UpdateRatio(ratio)
                }
            }
            None => {
                return;
            }
        };

        match action {
            TransitionAction::FadeOutComplete { next_music } => match next_music {
                Some(request) => {
                    self.finish_music(MusicStopReason::Replaced, Some(request.id.clone()));
                    self.queue_music_start(request);
                }
                None => self.finish_music(MusicStopReason::Faded, None),
            },
            TransitionAction::GapComplete { next_music } => {
                self.active_transition = None;
                self.start_music(next_music);
            }
            TransitionAction::FadeInComplete => {
                self.active_transition = None;
                self.set_music_ratio(1.0);
            }
            TransitionAction::UpdateRatio(ratio) => {
                self.set_music_ratio(ratio);
            }
        }
    }

    fn has_pending_music(&self) -> bool {
        matches!(
            self.active_transition,
            Some(MusicTransition::Gap { .. }) | Some(MusicTransition::FadeIn { .. })
        )
    }

    fn publish_runtime_state(&self) {
        runtime::set_music_playing(self.active_music.is_some() || self.has_pending_music());
    }

    fn tick_playback_state(&mut self, dt: f32) {
        self.tick_music_completion(dt);
        if self.active_music.is_some() || self.has_pending_music() {
            self.tick_transition(dt);
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

        self.tick_playback_state(dt);

        let commands = command_queue::drain_audio_commands();
        for cmd in commands {
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
                } => {
                    self.play_varied_sfx(&sounds, volume, pitch_variation, volume_variation);
                }
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
                } => {
                    self.play_loop(handle, &sounds, volume, pitch_variation, volume_variation);
                }
                #[cfg(feature = "editor")]
                AudioCommand::StopTrackedPreview(handle) => self.stop_tracked_preview(handle),
                AudioCommand::StopLoop(handle) => self.stop_loop(handle),
            }
        }
        self.publish_runtime_state();
    }
}
