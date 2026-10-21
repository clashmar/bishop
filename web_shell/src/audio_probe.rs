use bishop::audio::AUDIO_SAMPLE_RATE;

const NOTE_FRAMES: usize = AUDIO_SAMPLE_RATE as usize / 8;
const TONE_AMPLITUDE: f32 = 0.2;

#[derive(Default)]
struct ToneGenerator {
    remaining: usize,
    phase: f32,
}

impl ToneGenerator {
    fn trigger(&mut self) {
        if self.remaining == 0 {
            self.phase = 0.0;
        }
        self.remaining = NOTE_FRAMES;
    }

    fn cancel(&mut self) {
        self.remaining = 0;
    }

    fn render(&mut self, frames: &mut [[f32; 2]]) {
        for frame in frames {
            let sample = if self.remaining == 0 {
                0.0
            } else {
                let fade = self.remaining as f32 / NOTE_FRAMES as f32;
                let sample = self.phase.sin() * TONE_AMPLITUDE * fade;
                self.phase += std::f32::consts::TAU * 440.0 / AUDIO_SAMPLE_RATE as f32;
                self.remaining -= 1;
                sample
            };
            *frame = [sample, sample];
        }
    }
}

#[cfg(target_os = "emscripten")]
mod browser {
    use std::sync::{Arc, Mutex};

    use bishop::audio::{AudioBackend, PlatformAudioBackend};

    use super::ToneGenerator;

    pub(crate) struct AudioProbe {
        tone: Arc<Mutex<ToneGenerator>>,
        backend: Option<PlatformAudioBackend>,
    }

    impl AudioProbe {
        pub(crate) fn new() -> Self {
            Self {
                tone: Arc::new(Mutex::new(ToneGenerator::default())),
                backend: None,
            }
        }

        pub(crate) fn trigger(&mut self) {
            if self.backend.is_none() {
                let tone = Arc::clone(&self.tone);
                self.backend = Some(PlatformAudioBackend::start(move |frames| {
                    if let Ok(mut tone) = tone.lock() {
                        tone.render(frames);
                    } else {
                        frames.fill([0.0; 2]);
                    }
                }));
            } else {
                crate::emscripten::run_script_int("window.bishopAudioResume?.() ? 1 : 0");
            }
            if let Ok(mut tone) = self.tone.lock() {
                tone.trigger();
            }
        }

        pub(crate) fn cancel(&mut self) {
            if let Ok(mut tone) = self.tone.lock() {
                tone.cancel();
            }
        }
    }
}

#[cfg(target_os = "emscripten")]
pub(crate) use browser::AudioProbe;

#[cfg(test)]
mod tests {
    use super::{NOTE_FRAMES, TONE_AMPLITUDE, ToneGenerator};

    #[test]
    fn tone_without_trigger_renders_silence() {
        let mut tone = ToneGenerator::default();
        let mut frames = [[1.0, 1.0]; 256];
        tone.render(&mut frames);
        assert!(frames.iter().all(|frame| *frame == [0.0, 0.0]));
    }

    #[test]
    fn tone_after_trigger_renders_bounded_stereo_samples() {
        let mut tone = ToneGenerator::default();
        tone.trigger();
        let mut frames = [[0.0; 2]; 256];
        tone.render(&mut frames);
        assert!(frames.iter().any(|frame| frame[0].abs() > 0.01));
        assert!(frames.iter().all(|frame| {
            frame[0] == frame[1] && frame[0].is_finite() && frame[0].abs() <= TONE_AMPLITUDE
        }));
    }

    #[test]
    fn tone_after_note_duration_returns_to_silence() {
        let mut tone = ToneGenerator::default();
        tone.trigger();
        let mut frames = vec![[0.0; 2]; NOTE_FRAMES + 256];
        tone.render(&mut frames);
        assert!(frames[NOTE_FRAMES..].iter().all(|frame| *frame == [0.0, 0.0]));
    }

    #[test]
    fn tone_after_cancel_discards_remaining_samples() {
        let mut tone = ToneGenerator::default();
        tone.trigger();
        tone.cancel();
        let mut frames = [[1.0; 2]; 256];
        tone.render(&mut frames);
        assert!(frames.iter().all(|frame| *frame == [0.0, 0.0]));
    }

    #[test]
    fn tone_during_retrigger_restarts_duration_without_phase_jump() {
        let mut tone = ToneGenerator::default();
        tone.trigger();
        let mut lead = [[0.0; 2]; 128];
        tone.render(&mut lead);
        let previous = lead[127][0];
        tone.trigger();
        let mut next = [[0.0; 2]; 1];
        tone.render(&mut next);
        assert!((next[0][0] - previous).abs() < 0.02);

        let mut tail = vec![[0.0; 2]; NOTE_FRAMES + 256];
        tone.render(&mut tail);
        assert!(tail[NOTE_FRAMES - 1..].iter().all(|frame| *frame == [0.0, 0.0]));
    }
}
