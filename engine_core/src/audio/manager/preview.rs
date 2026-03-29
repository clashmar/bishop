use super::*;
#[cfg(feature = "editor")]
use rand::Rng;

#[cfg(feature = "editor")]
pub(super) struct TrackedPreview {
    pub(super) handle: PreviewSignal,
    pub(super) expires_at: f32,
}

#[cfg(feature = "editor")]
pub(super) struct PendingPreview {
    pub(super) sound_id: String,
    pub(super) volume: f32,
    pub(super) pitch: f32,
    pub(super) looping: bool,
    pub(super) timeout: f32,
}

#[cfg(feature = "editor")]
pub(super) enum PreviewSignal {
    OneShot(PreviewHandle),
    Loop(LoopHandle),
}

#[cfg(feature = "editor")]
pub(super) struct TrackedPreviewSpec<'a> {
    pub(super) sounds: &'a [String],
    pub(super) volume: f32,
    pub(super) pitch_variation: f32,
    pub(super) volume_variation: f32,
    pub(super) looping: bool,
    pub(super) timeout: f32,
}

impl AudioManager {
    #[cfg(feature = "editor")]
    fn start_tracked_preview(
        &mut self,
        handle_key: u64,
        frames: Arc<Frames<[f32; 2]>>,
        pending: PendingPreview,
    ) {
        let expires_at = self.preview_time + pending.timeout.max(0.0);

        let signal = if pending.looping {
            let mut signal = Gain::new(Speed::new(Cycle::new(frames)));
            signal.set_amplitude_ratio(pending.volume);
            let mut handle = self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
            handle
                .control::<Speed<Cycle<[f32; 2]>>, _>()
                .set_speed(pending.pitch);
            PreviewSignal::Loop(handle)
        } else {
            let mut signal = Gain::new(Speed::new(FramesSignal::from(frames)));
            signal.set_amplitude_ratio(pending.volume);
            let mut handle = self.sfx_group.control::<Mixer<[f32; 2]>, _>().play(signal);
            handle
                .control::<Speed<FramesSignal<[f32; 2]>>, _>()
                .set_speed(pending.pitch);
            PreviewSignal::OneShot(handle)
        };

        self.tracked_previews.insert(
            handle_key,
            TrackedPreview {
                handle: signal,
                expires_at,
            },
        );
        #[cfg(test)]
        self.test_state.started_tracked_preview_playbacks.insert(
            handle_key,
            StartedTrackedPreviewPlayback {
                id: pending.sound_id,
                volume: pending.volume,
                pitch: pending.pitch,
                looping: pending.looping,
            },
        );
    }

    #[cfg(feature = "editor")]
    pub(super) fn play_tracked_preview(&mut self, handle_key: u64, spec: TrackedPreviewSpec<'_>) {
        self.stop_tracked_preview(handle_key);
        let Some(sound_id) = Self::pick_sound(spec.sounds).map(str::to_owned) else {
            return;
        };
        let final_volume = Self::apply_variation(spec.volume, spec.volume_variation);
        let final_pitch = (1.0
            + rand::thread_rng().gen_range(-spec.pitch_variation..=spec.pitch_variation))
        .max(0.1);

        let pending = PendingPreview {
            sound_id: sound_id.clone(),
            volume: final_volume,
            pitch: final_pitch,
            looping: spec.looping,
            timeout: spec.timeout,
        };

        let Some(frames) = self.cached_frames(&sound_id) else {
            self.queue_tracked_preview(handle_key, pending);
            return;
        };

        self.start_tracked_preview(handle_key, frames, pending);
    }

    #[cfg(feature = "editor")]
    pub(super) fn stop_tracked_preview(&mut self, handle_key: u64) {
        self.pending_previews.remove(&handle_key);
        #[cfg(test)]
        self.test_state
            .started_tracked_preview_playbacks
            .remove(&handle_key);
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
    fn queue_tracked_preview(&mut self, handle_key: u64, pending: PendingPreview) {
        let sound_id = pending.sound_id.clone();
        self.pending_previews.insert(handle_key, pending);
        self.queue_sound_load(&sound_id);
    }

    #[cfg(feature = "editor")]
    pub(super) fn resolve_pending_tracked_previews(&mut self) {
        let pending_handles = self.pending_previews.keys().copied().collect::<Vec<_>>();

        for handle_key in pending_handles {
            let Some(pending) = self.pending_previews.get(&handle_key) else {
                continue;
            };
            if self.pending_loads.contains_key(&pending.sound_id) {
                continue;
            }

            let Some(frames) = self.cached_frames(&pending.sound_id) else {
                let _ = self.pending_previews.remove(&handle_key);
                continue;
            };

            let Some(pending) = self.pending_previews.remove(&handle_key) else {
                continue;
            };

            self.start_tracked_preview(handle_key, frames, pending);
        }
    }

    #[cfg(feature = "editor")]
    pub(super) fn cleanup_tracked_previews(&mut self) {
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
}
