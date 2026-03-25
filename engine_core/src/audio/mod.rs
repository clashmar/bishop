pub mod command_queue;
pub mod loader;

pub use command_queue::{AudioCommand, push_audio_command};
pub use loader::load_wav;

use crate::task::BackgroundService;
use bishop::audio::AudioBackend;
use oddio::{Cycle, Frames, FramesSignal, Gain, Handle, Mixer, Stop};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Active fade-out state for a music track.
struct FadeOut {
    remaining: f32,
    duration: f32,
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
    active_music: Option<Handle<Stop<Cycle<[f32; 2]>>>>,
    /// Active fade-out state.
    active_fade: Option<FadeOut>,
    /// Decoded audio cache, keyed by sound ID.
    sound_cache: HashMap<String, Arc<Frames<[f32; 2]>>>,
    /// Reference counts tracking how many `AudioSource` components reference each sound ID.
    ref_counts: HashMap<String, usize>,
    /// Sound IDs loaded via `preload()` from Lua; pinned sounds are never auto-evicted.
    pinned: HashSet<String>,
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
        let keep_alive: Box<dyn Send + 'static> = Box::new(B::start(move |frames: &mut [[f32; 2]]| {
            oddio::run(&root_signal, SAMPLE_RATE, frames);
        }));

        Self {
            _keep_alive: keep_alive,
            _root: root_handle,
            music_group: music_group_handle,
            sfx_group: sfx_group_handle,
            active_music: None,
            active_fade: None,
            sound_cache: HashMap::new(),
            ref_counts: HashMap::new(),
            pinned: HashSet::new(),
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

    /// Starts playing a looping music track. Any existing track is stopped immediately.
    fn play_music(&mut self, id: &str) {
        self.stop_music();
        let Some(frames) = self.load_or_cached(id) else {
            return;
        };
        let cycle = Cycle::new(frames);
        // Play the looping cycle into the inner Mixer of the music group.
        // Signal chain: Stop<Gain<Mixer>> — Mixer is accessible via the filter chain.
        let track_handle = self
            .music_group
            .control::<Mixer<[f32; 2]>, _>()
            .play(cycle);
        self.active_music = Some(track_handle);
    }

    /// Stops the active music track immediately.
    fn stop_music(&mut self) {
        if let Some(ref mut handle) = self.active_music {
            handle.control::<Stop<Cycle<[f32; 2]>>, _>().stop();
        }
        self.active_music = None;
        self.active_fade = None;
    }

    /// Begins a fade-out of the active music over `duration` seconds.
    fn fade_music(&mut self, duration: f32) {
        if self.active_music.is_some() {
            self.active_fade = Some(FadeOut {
                remaining: duration,
                duration,
            });
        }
    }

    /// Plays a one-shot SFX by ID. Fire and forget.
    fn play_sfx(&mut self, id: &str) {
        let Some(frames) = self.load_or_cached(id) else {
            return;
        };
        let signal = FramesSignal::from(frames);
        self.sfx_group
            .control::<Mixer<[f32; 2]>, _>()
            .play(signal);
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

    /// Advances the active fade-out by `dt` seconds.
    fn tick_fade(&mut self, dt: f32) {
        let finished = if let Some(ref mut fade) = self.active_fade {
            fade.remaining -= dt;
            if fade.remaining <= 0.0 {
                true
            } else {
                let ratio = (fade.remaining / fade.duration).clamp(0.0, 1.0);
                let linear = self.master_volume * self.music_volume * ratio;
                self.music_group
                    .control::<Gain<Mixer<[f32; 2]>>, _>()
                    .set_amplitude_ratio(linear);
                false
            }
        } else {
            false
        };

        if finished {
            self.stop_music();
            // Restore music volume after stopping.
            self.apply_music_gain();
        }
    }
}

impl BackgroundService for AudioManager {
    /// Drains the audio command queue and advances any active fade. Must not block.
    fn poll(&mut self, dt: f32) {
        let commands = command_queue::drain_audio_commands();
        for cmd in commands {
            match cmd {
                AudioCommand::PlayMusic(id) => self.play_music(&id),
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
            }
        }
        self.tick_fade(dt);
    }
}
