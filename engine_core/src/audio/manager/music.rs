use super::*;

impl AudioManager {
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
    pub(super) fn play_music(&mut self, request: PlayMusicRequest) {
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
    pub(super) fn stop_music(&mut self) {
        if self.active_music.is_some() {
            self.finish_music(MusicStopReason::Stopped, None);
            return;
        }

        self.active_transition = None;
        self.set_music_ratio(1.0);
    }

    /// Begins a fade-out of the active music over `duration` seconds.
    pub(super) fn fade_music(&mut self, duration: f32) {
        if self.active_music.is_some() {
            self.begin_fade_out(duration.max(0.0), None);
            return;
        }

        self.active_transition = None;
        self.set_music_ratio(1.0);
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

    pub(super) fn publish_runtime_state(&self) {
        runtime::set_music_playing(self.active_music.is_some() || self.has_pending_music());
    }

    pub(super) fn tick_playback_state(&mut self, dt: f32) {
        self.tick_music_completion(dt);
        if self.active_music.is_some() || self.has_pending_music() {
            self.tick_transition(dt);
        }
    }
}
