use super::*;
#[cfg(feature = "editor")]
use rand::Rng;

impl AudioManager {
    #[cfg(feature = "editor")]
    pub(super) fn play_tracked_preview(&mut self, handle_key: u64, spec: TrackedPreviewSpec<'_>) {
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
    pub(super) fn stop_tracked_preview(&mut self, handle_key: u64) {
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
