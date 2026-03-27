use super::*;
use rand::Rng;

impl AudioManager {
    /// Plays a one-shot SFX by ID. Fire and forget.
    pub(super) fn play_sfx(&mut self, id: &str) {
        let Some(frames) = self.load_or_cached(id) else {
            return;
        };
        let signal = FramesSignal::from(frames);
        self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
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
    pub(super) fn play_loop(
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
    pub(super) fn stop_loop(&mut self, handle_key: u64) {
        if let Some(mut handle) = self.active_loops.remove(&handle_key) {
            handle
                .control::<Stop<Gain<Speed<Cycle<[f32; 2]>>>>, _>()
                .stop();
        }
    }
}
