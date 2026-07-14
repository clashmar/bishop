use super::*;
use rand::Rng;

impl AudioManager {
    /// Plays a one-shot SFX by ID. Fire and forget.
    pub(super) fn play_sfx(&mut self, id: &str) {
        let Some(frames) = self.cached_frames(id) else {
            self.queue_one_shot(id, PendingOneShot::Plain);
            return;
        };
        self.play_one_shot_frames(id, frames, 1.0, 1.0);
    }

    /// Applies a random variation to `base`, clamped to [0.0, 1.0].
    /// Returns `base` unchanged when `variation` is zero.
    pub(super) fn apply_variation(base: f32, variation: f32) -> f32 {
        if variation == 0.0 {
            return base;
        }
        let delta = rand::thread_rng().gen_range(-variation..=variation);
        (base + delta).clamp(0.0, 1.0)
    }

    /// Selects a random element from `sounds`, returning `None` when the slice is empty.
    pub(super) fn pick_sound(sounds: &[String]) -> Option<&str> {
        if sounds.is_empty() {
            return None;
        }
        let idx = rand::thread_rng().gen_range(0..sounds.len());
        Some(&sounds[idx])
    }

    /// Plays a one-shot sound chosen randomly from `sounds`, with optional pitch and volume variation.
    pub(super) fn play_varied_sfx(
        &mut self,
        sounds: &[String],
        volume: f32,
        pitch_variation: f32,
        volume_variation: f32,
    ) {
        let Some(id) = Self::pick_sound(sounds) else {
            return;
        };
        let final_volume = Self::apply_variation(volume, volume_variation);
        let final_pitch =
            (1.0 + rand::thread_rng().gen_range(-pitch_variation..=pitch_variation)).max(0.1);
        let Some(frames) = self.cached_frames(id) else {
            self.queue_one_shot(
                id,
                PendingOneShot::Varied {
                    volume: final_volume,
                    pitch: final_pitch,
                },
            );
            return;
        };
        self.play_one_shot_frames(id, frames, final_volume, final_pitch);
    }

    /// Starts a looping sound for the given key, replacing any existing loop for that key.
    pub(super) fn play_loop(
        &mut self,
        key: AudioLoopKey,
        sounds: &[String],
        volume: f32,
        pitch_variation: f32,
        volume_variation: f32,
    ) {
        self.stop_loop_key(&key, None);
        let Some(id) = Self::pick_sound(sounds) else {
            return;
        };
        let final_volume = Self::apply_variation(volume, volume_variation);
        let final_pitch =
            (1.0 + rand::thread_rng().gen_range(-pitch_variation..=pitch_variation)).max(0.1);
        let Some(frames) = self.cached_frames(id) else {
            self.queue_loop(
                key,
                PendingLoop {
                    sound_id: id.to_owned(),
                    volume: final_volume,
                    pitch: final_pitch,
                },
            );
            return;
        };
        self.start_loop_frames(key, id, frames, final_volume, final_pitch);
    }

    /// Stops all looping sounds associated with `owner`, if any exist.
    pub(super) fn stop_loops(&mut self, owner: AudioPlaybackOwner, fade_out: Option<f32>) {
        let owner = &owner;
        let pending_keys = self
            .pending_loops
            .keys()
            .filter(|key| &key.owner == owner)
            .cloned()
            .collect::<Vec<_>>();
        for key in pending_keys {
            self.pending_loops.remove(&key);
        }

        let active_keys = self
            .active_loops
            .keys()
            .filter(|key| &key.owner == owner)
            .cloned()
            .collect::<Vec<_>>();
        for key in active_keys {
            self.stop_loop_key(&key, fade_out);
        }
    }

    pub(super) fn tick_loop_fades(&mut self, dt: f32) {
        let mut finished_keys = Vec::new();
        for (key, playback) in &mut self.active_loops {
            let Some(fade) = playback.fade.as_mut() else {
                continue;
            };
            fade.remaining = (fade.remaining - dt).max(0.0);
            let ratio = if fade.duration == 0.0 {
                0.0
            } else {
                playback.volume * (fade.remaining / fade.duration)
            };
            playback
                .handle
                .control::<Gain<Speed<Cycle<[f32; 2]>>>, _>()
                .set_amplitude_ratio(ratio);
            if fade.remaining == 0.0 {
                finished_keys.push(key.clone());
            }
        }

        for key in finished_keys {
            self.finish_loop_stop(&key);
        }
    }

    pub(super) fn resolve_pending_sfx(&mut self) {
        let pending_one_shot_ids = self.pending_one_shots.keys().cloned().collect::<Vec<_>>();
        for id in pending_one_shot_ids {
            if self.pending_loads.contains_key(&id) {
                continue;
            }

            let Some(frames) = self.cached_frames(&id) else {
                let _ = self.pending_one_shots.remove(&id);
                continue;
            };

            let Some(requests) = self.pending_one_shots.remove(&id) else {
                continue;
            };

            for request in requests {
                match request {
                    PendingOneShot::Plain => {
                        self.play_one_shot_frames(&id, frames.clone(), 1.0, 1.0)
                    }
                    PendingOneShot::Varied { volume, pitch } => {
                        self.play_one_shot_frames(&id, frames.clone(), volume, pitch)
                    }
                }
            }
        }

        let pending_loop_keys = self.pending_loops.keys().cloned().collect::<Vec<_>>();
        for key in pending_loop_keys {
            let Some(pending) = self.pending_loops.get(&key) else {
                continue;
            };
            if self.pending_loads.contains_key(&pending.sound_id) {
                continue;
            }

            let Some(frames) = self.cached_frames(&pending.sound_id) else {
                let _ = self.pending_loops.remove(&key);
                continue;
            };

            let Some(pending) = self.pending_loops.remove(&key) else {
                continue;
            };

            self.start_loop_frames(key, &pending.sound_id, frames, pending.volume, pending.pitch);
        }
    }

    fn play_one_shot_frames(
        &mut self,
        _id: &str,
        frames: Arc<Frames<[f32; 2]>>,
        volume: f32,
        pitch: f32,
    ) {
        let mut signal = Gain::new(Speed::new(FramesSignal::from(frames)));
        signal.set_amplitude_ratio(volume);
        let mut handle = self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
        handle
            .control::<Speed<FramesSignal<[f32; 2]>>, _>()
            .set_speed(pitch);
        #[cfg(test)]
        self.test_state
            .started_one_shot_playbacks
            .push(StartedOneShotPlayback {
                id: _id.to_owned(),
                volume,
                pitch,
            });
    }

    fn start_loop_frames(
        &mut self,
        key: AudioLoopKey,
        id: &str,
        frames: Arc<Frames<[f32; 2]>>,
        volume: f32,
        pitch: f32,
    ) {
        self.claim_sound(id);
        let mut signal = Gain::new(Speed::new(Cycle::new(frames)));
        signal.set_amplitude_ratio(volume);
        let mut handle = self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
        handle
            .control::<Speed<Cycle<[f32; 2]>>, _>()
            .set_speed(pitch);
        self.active_loops.insert(
            key.clone(),
            ActiveLoopPlayback {
                sound_id: id.to_owned(),
                handle,
                volume,
                fade: None,
            },
        );
        #[cfg(test)]
        self.test_state
            .active_loop_sound_ids
            .insert(key.clone(), id.to_owned());
        #[cfg(test)]
        self.test_state.started_loop_playbacks.insert(
            key,
            StartedLoopPlayback {
                id: id.to_owned(),
                volume,
                pitch,
            },
        );
    }

    fn queue_one_shot(&mut self, id: &str, request: PendingOneShot) {
        self.pending_one_shots
            .entry(id.to_owned())
            .or_default()
            .push(request);
        self.queue_sound_load(id);
    }

    fn queue_loop(&mut self, key: AudioLoopKey, pending: PendingLoop) {
        let sound_id = pending.sound_id.clone();
        self.pending_loops.insert(key, pending);
        self.queue_sound_load(&sound_id);
    }

    fn stop_loop_key(&mut self, key: &AudioLoopKey, fade_out: Option<f32>) {
        self.pending_loops.remove(key);
        if let Some(playback) = self.active_loops.get_mut(key)
            && Self::begin_loop_stop(playback, fade_out)
        {
            self.finish_loop_stop(key);
        }
    }

    fn begin_loop_stop(playback: &mut ActiveLoopPlayback, fade_out: Option<f32>) -> bool {
        match fade_out.filter(|duration| *duration > 0.0) {
            Some(duration) => {
                playback.fade = Some(LoopFade {
                    remaining: duration,
                    duration,
                });
                false
            }
            None => true,
        }
    }

    fn finish_loop_stop(&mut self, key: &AudioLoopKey) {
        #[cfg(test)]
        self.test_state.active_loop_sound_ids.remove(key);
        #[cfg(test)]
        self.test_state.started_loop_playbacks.remove(key);
        if let Some(mut playback) = self.active_loops.remove(key) {
            playback
                .handle
                .control::<Stop<Gain<Speed<Cycle<[f32; 2]>>>>, _>()
                .stop();
            self.release_claimed_sound(&playback.sound_id);
        }
    }
}
