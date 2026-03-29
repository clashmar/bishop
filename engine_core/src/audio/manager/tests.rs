use super::*;
use crate::audio::command_queue::{drain_audio_commands, push_audio_command};
use crate::audio::runtime;
use crate::audio::{AudioCommand, AudioDiagnosticsEntry, PlayMusicRequest};
use crate::task::BackgroundService;
use bishop::audio::AudioBackend;
use oddio::Frames;

struct TestBackend;

impl AudioBackend for TestBackend {
    fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(_render_fn: F) -> Self
    where
        Self: Sized,
    {
        Self
    }
}

fn seeded_manager() -> AudioManager {
    runtime::reset_for_tests();
    let _ = drain_audio_commands();

    let mut manager = AudioManager::new::<TestBackend>();
    manager.sound_cache.insert(
        "preview/click".to_string(),
        Frames::from_slice(44_100, &[[0.0, 0.0]]),
    );
    manager.sound_cache.insert(
        "music/intro".to_string(),
        Frames::from_slice(10, &[[0.0, 0.0]; 10]),
    );
    manager.sound_cache.insert(
        "music/next".to_string(),
        Frames::from_slice(10, &[[0.0, 0.0]; 20]),
    );
    manager
}

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn diagnostics_snapshot_includes_cached_only_entries() {
    let manager = seeded_manager();

    let snapshot = manager.diagnostics_snapshot();

    assert_eq!(snapshot.cached_sound_count, 3);
    assert_eq!(snapshot.loading_sound_count, 0);
    assert_eq!(snapshot.pinned_sound_count, 0);
    assert_eq!(snapshot.ref_count_entry_count, 0);
    assert_eq!(snapshot.entries.len(), 3);
    assert_eq!(
        snapshot.entries,
        vec![
            AudioDiagnosticsEntry {
                id: "music/intro".to_string(),
                cached: true,
                loading: false,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "music/next".to_string(),
                cached: true,
                loading: false,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "preview/click".to_string(),
                cached: true,
                loading: false,
                pinned: false,
                ref_count: 0,
            },
        ]
    );
}

#[test]
fn diagnostics_snapshot_includes_pinned_and_ref_counted_entries() {
    let mut manager = seeded_manager();
    manager.pinned.insert("pinned/only".to_string());
    manager.ref_counts.insert("ref/only".to_string(), 2);
    manager.ref_counts.insert("shared".to_string(), 1);
    manager.sound_cache.insert(
        "shared".to_string(),
        Frames::from_slice(44_100, &[[0.0, 0.0]]),
    );

    let snapshot = manager.diagnostics_snapshot();

    assert_eq!(snapshot.cached_sound_count, 4);
    assert_eq!(snapshot.loading_sound_count, 0);
    assert_eq!(snapshot.pinned_sound_count, 1);
    assert_eq!(snapshot.ref_count_entry_count, 2);
    assert_eq!(snapshot.entries.len(), 6);
    assert_eq!(
        snapshot.entries,
        vec![
            AudioDiagnosticsEntry {
                id: "music/intro".to_string(),
                cached: true,
                loading: false,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "music/next".to_string(),
                cached: true,
                loading: false,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "pinned/only".to_string(),
                cached: false,
                loading: false,
                pinned: true,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "preview/click".to_string(),
                cached: true,
                loading: false,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "ref/only".to_string(),
                cached: false,
                loading: false,
                pinned: false,
                ref_count: 2,
            },
            AudioDiagnosticsEntry {
                id: "shared".to_string(),
                cached: true,
                loading: false,
                pinned: false,
                ref_count: 1,
            },
        ]
    );
}

#[test]
fn diagnostics_snapshot_entries_are_sorted_by_sound_id() {
    let mut manager = seeded_manager();
    manager.sound_cache.clear();
    manager.sound_cache.insert(
        "zeta".to_string(),
        Frames::from_slice(44_100, &[[0.0, 0.0]]),
    );
    manager.sound_cache.insert(
        "alpha".to_string(),
        Frames::from_slice(44_100, &[[0.0, 0.0]]),
    );
    manager.pinned.insert("middle".to_string());
    manager.ref_counts.insert("beta".to_string(), 1);

    let snapshot = manager.diagnostics_snapshot();

    let ids: Vec<String> = snapshot.entries.into_iter().map(|entry| entry.id).collect();
    assert_eq!(
        ids,
        vec![
            "alpha".to_string(),
            "beta".to_string(),
            "middle".to_string(),
            "zeta".to_string(),
        ]
    );
}

#[test]
fn diagnostics_snapshot_includes_loading_entries() {
    let mut manager = seeded_manager();
    manager.sound_cache.remove("music/next");
    manager.queue_sound_load("music/next");

    let snapshot = manager.diagnostics_snapshot();

    assert_eq!(snapshot.cached_sound_count, 2);
    assert_eq!(snapshot.loading_sound_count, 1);
    assert_eq!(snapshot.pinned_sound_count, 0);
    assert_eq!(snapshot.ref_count_entry_count, 0);
    assert_eq!(
        snapshot.entries,
        vec![
            AudioDiagnosticsEntry {
                id: "music/intro".to_string(),
                cached: true,
                loading: false,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "music/next".to_string(),
                cached: false,
                loading: true,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "preview/click".to_string(),
                cached: true,
                loading: false,
                pinned: false,
                ref_count: 0,
            },
        ]
    );
}

#[test]
fn one_shot_music_completion_updates_runtime_and_emits_event() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: false,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);

    assert!(runtime::is_music_playing());
    assert!(runtime::drain_audio_events().is_empty());

    manager.poll(1.0);

    assert!(!runtime::is_music_playing());
    let events = runtime::drain_audio_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "music/intro");
    assert_eq!(events[0].reason, runtime::MusicStopReason::Completed);
    assert_eq!(events[0].next_id, None);
}

#[test]
fn fade_then_start_replaces_current_track_after_fade() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);
    let _ = runtime::drain_audio_events();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/next".to_string(),
        looping: true,
        fade_out: 0.5,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.25);

    assert!(runtime::is_music_playing());
    assert!(runtime::drain_audio_events().is_empty());

    manager.poll(0.5);

    assert!(runtime::is_music_playing());
    let events = runtime::drain_audio_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "music/intro");
    assert_eq!(events[0].reason, runtime::MusicStopReason::Replaced);
    assert_eq!(events[0].next_id.as_deref(), Some("music/next"));
}

#[test]
fn stop_music_cancels_pending_replacement_and_emits_stopped_event() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);
    let _ = runtime::drain_audio_events();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/next".to_string(),
        looping: true,
        fade_out: 1.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.25);
    push_audio_command(AudioCommand::StopMusic);
    manager.poll(0.0);

    assert!(!runtime::is_music_playing());
    let events = runtime::drain_audio_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "music/intro");
    assert_eq!(events[0].reason, runtime::MusicStopReason::Stopped);
    assert_eq!(events[0].next_id, None);
}

#[test]
fn fresh_start_gap_and_fade_in_keep_music_playing_until_full_volume() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.5,
        fade_in: 0.5,
    }));
    manager.poll(0.0);

    assert!(runtime::is_music_playing());
    assert!(manager.active_music.is_none());
    assert!(matches!(
        manager.active_transition,
        Some(MusicTransition::Gap { .. })
    ));

    manager.poll(0.5);

    assert!(runtime::is_music_playing());
    assert!(manager.active_music.is_some());
    assert!(matches!(
        manager.active_transition,
        Some(MusicTransition::FadeIn { .. })
    ));
    assert_approx_eq(manager.music_ratio, 0.0);

    manager.poll(0.25);
    assert_approx_eq(manager.music_ratio, 0.5);

    manager.poll(0.25);
    assert_approx_eq(manager.music_ratio, 1.0);
    assert!(manager.active_transition.is_none());
}

#[test]
fn replacement_gap_emits_stop_event_before_next_track_starts() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);
    let _ = runtime::drain_audio_events();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/next".to_string(),
        looping: true,
        fade_out: 0.5,
        gap: 0.5,
        fade_in: 0.5,
    }));
    manager.poll(0.0);
    manager.poll(0.5);

    assert!(runtime::is_music_playing());
    assert!(manager.active_music.is_none());
    assert!(matches!(
        manager.active_transition,
        Some(MusicTransition::Gap { .. })
    ));

    let events = runtime::drain_audio_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "music/intro");
    assert_eq!(events[0].reason, runtime::MusicStopReason::Replaced);
    assert_eq!(events[0].next_id.as_deref(), Some("music/next"));

    manager.poll(0.5);
    assert!(manager.active_music.is_some());
    assert!(matches!(
        manager.active_transition,
        Some(MusicTransition::FadeIn { .. })
    ));
    assert_approx_eq(manager.music_ratio, 0.0);
}

#[test]
fn stop_music_cancels_pending_gap_before_next_track_starts() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 1.0,
        fade_in: 0.5,
    }));
    manager.poll(0.0);

    assert!(runtime::is_music_playing());
    assert!(manager.active_music.is_none());

    push_audio_command(AudioCommand::StopMusic);
    manager.poll(0.0);

    assert!(!runtime::is_music_playing());
    assert!(manager.active_music.is_none());
    assert!(manager.active_transition.is_none());
    assert!(runtime::drain_audio_events().is_empty());
}

#[test]
fn replacement_during_fade_in_starts_fade_out_from_current_ratio() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 1.0,
    }));
    manager.poll(0.0);
    manager.poll(0.25);
    assert_approx_eq(manager.music_ratio, 0.25);

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/next".to_string(),
        looping: true,
        fade_out: 0.5,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);

    assert!(matches!(
        manager.active_transition,
        Some(MusicTransition::FadeOut { .. })
    ));
    assert_approx_eq(manager.music_ratio, 0.25);

    manager.poll(0.25);
    assert_approx_eq(manager.music_ratio, 0.125);
}

#[test]
fn uncached_music_waits_for_background_load_before_starting() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/cold".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);

    assert!(runtime::is_music_playing());
    assert!(manager.active_music.is_none());
    assert!(manager.pending_music.is_some());
    assert!(manager.pending_loads.contains_key("music/cold"));

    manager.complete_load_for_test("music/cold", Frames::from_slice(10, &[[0.0, 0.0]; 10]));
    manager.poll(0.0);

    assert!(manager.pending_music.is_none());
    assert!(manager.pending_loads.is_empty());
    assert!(manager.active_music.is_some());
}

#[test]
fn uncached_replacement_keeps_current_track_until_loaded() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);
    let _ = runtime::drain_audio_events();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/cold".to_string(),
        looping: true,
        fade_out: 0.5,
        gap: 0.25,
        fade_in: 0.25,
    }));
    manager.poll(0.0);

    assert!(runtime::is_music_playing());
    assert!(manager.active_music.is_some());
    assert!(manager.pending_music.is_some());
    assert!(manager.active_transition.is_none());
    assert!(runtime::drain_audio_events().is_empty());

    manager.complete_load_for_test("music/cold", Frames::from_slice(10, &[[0.0, 0.0]; 10]));
    manager.poll(0.0);

    assert!(matches!(
        manager.active_transition,
        Some(MusicTransition::FadeOut { .. })
    ));
    assert!(manager.active_music.is_some());
    assert!(runtime::drain_audio_events().is_empty());

    manager.poll(0.5);

    let events = runtime::drain_audio_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "music/intro");
    assert_eq!(events[0].reason, runtime::MusicStopReason::Replaced);
    assert_eq!(events[0].next_id.as_deref(), Some("music/cold"));
    assert!(matches!(
        manager.active_transition,
        Some(MusicTransition::Gap { .. })
    ));
}

#[test]
fn stop_music_cancels_pending_initial_load_without_event() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/cold".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);

    push_audio_command(AudioCommand::StopMusic);
    manager.poll(0.0);

    assert!(!runtime::is_music_playing());
    assert!(manager.active_music.is_none());
    assert!(manager.pending_music.is_none());
    assert!(manager.active_transition.is_none());
    assert!(runtime::drain_audio_events().is_empty());

    manager.complete_load_for_test("music/cold", Frames::from_slice(10, &[[0.0, 0.0]; 10]));
    manager.poll(0.0);

    assert!(manager.active_music.is_none());
    assert!(runtime::drain_audio_events().is_empty());
}

#[test]
fn newer_pending_music_request_wins_when_multiple_loads_finish() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/cold-a".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/cold-b".to_string(),
        looping: true,
        fade_out: 0.0,
        gap: 0.0,
        fade_in: 0.0,
    }));
    manager.poll(0.0);

    manager.complete_load_for_test("music/cold-a", Frames::from_slice(10, &[[0.0, 0.0]; 10]));
    manager.poll(0.0);
    assert!(manager.active_music.is_none());

    manager.complete_load_for_test("music/cold-b", Frames::from_slice(10, &[[0.0, 0.0]; 10]));
    manager.poll(0.0);

    match manager.active_music.as_ref() {
        Some(ActiveMusic::Looping { id, .. }) => assert_eq!(id, "music/cold-b"),
        Some(ActiveMusic::OneShot { id, .. }) => assert_eq!(id, "music/cold-b"),
        None => panic!("expected active music"),
    }
    assert!(manager.sound_cache.contains_key("music/cold-a"));
    assert!(manager.sound_cache.contains_key("music/cold-b"));
}

#[test]
fn uncached_one_shot_queues_until_load_completes() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlaySfx("sfx/cold".to_string()));
    manager.poll(0.0);

    assert!(manager.pending_loads.contains_key("sfx/cold"));
    assert_eq!(manager.pending_one_shots.len(), 1);
    assert!(manager.pending_one_shots.contains_key("sfx/cold"));
    assert!(manager.test_state.started_one_shot_playbacks.is_empty());

    manager.complete_load_for_test("sfx/cold", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(!manager.pending_one_shots.contains_key("sfx/cold"));
    assert_eq!(
        manager.test_state.started_one_shot_playbacks,
        vec![StartedOneShotPlayback {
            id: "sfx/cold".to_string(),
            volume: 1.0,
            pitch: 1.0,
        }]
    );
}

#[test]
fn repeated_uncached_one_shots_remain_additive() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlaySfx("sfx/cold".to_string()));
    push_audio_command(AudioCommand::PlaySfx("sfx/cold".to_string()));
    manager.poll(0.0);

    assert_eq!(
        manager.pending_one_shots.get("sfx/cold").map(Vec::len),
        Some(2)
    );
    assert!(manager.test_state.started_one_shot_playbacks.is_empty());

    manager.complete_load_for_test("sfx/cold", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(!manager.pending_one_shots.contains_key("sfx/cold"));
    assert_eq!(
        manager.test_state.started_one_shot_playbacks,
        vec![
            StartedOneShotPlayback {
                id: "sfx/cold".to_string(),
                volume: 1.0,
                pitch: 1.0,
            },
            StartedOneShotPlayback {
                id: "sfx/cold".to_string(),
                volume: 1.0,
                pitch: 1.0,
            },
        ]
    );
}

#[test]
fn deferred_varied_sfx_preserves_captured_playback_parameters() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayVariedSfx {
        sounds: vec!["sfx/cold".to_string()],
        volume: 0.6,
        pitch_variation: 0.2,
        volume_variation: 0.1,
    });
    manager.poll(0.0);

    let pending = manager
        .pending_one_shots
        .get("sfx/cold")
        .and_then(|requests| requests.first());
    let Some(PendingOneShot::Varied { volume, pitch }) = pending else {
        panic!("expected deferred varied one-shot");
    };
    let pending_volume = *volume;
    let pending_pitch = *pitch;

    manager.complete_load_for_test("sfx/cold", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert_eq!(
        manager.test_state.started_one_shot_playbacks,
        vec![StartedOneShotPlayback {
            id: "sfx/cold".to_string(),
            volume: pending_volume,
            pitch: pending_pitch,
        }]
    );
}

#[test]
fn stop_loop_cancels_pending_cold_loop_before_playback_starts() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayLoop {
        handle: 41,
        sounds: vec!["loop/cold".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    assert!(manager.pending_loops.contains_key(&41));
    assert!(manager.active_loops.is_empty());

    push_audio_command(AudioCommand::StopLoop(41));
    manager.poll(0.0);

    assert!(!manager.pending_loops.contains_key(&41));

    manager.complete_load_for_test("loop/cold", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(!manager.active_loops.contains_key(&41));
}

#[test]
fn failed_cold_one_shot_load_clears_pending_state_without_playback() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlaySfx("sfx/fail".to_string()));
    manager.poll(0.0);

    assert!(manager.pending_one_shots.contains_key("sfx/fail"));
    assert!(manager.pending_loads.contains_key("sfx/fail"));

    manager.fail_load_for_test("sfx/fail", "boom");
    manager.poll(0.0);

    assert!(!manager.pending_one_shots.contains_key("sfx/fail"));
    assert!(!manager.pending_loads.contains_key("sfx/fail"));
    assert!(manager.test_state.started_one_shot_playbacks.is_empty());
}

#[test]
fn failed_load_clears_pending_runtime_requests() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlaySfx("sfx/missing".to_string()));
    push_audio_command(AudioCommand::PlayLoop {
        handle: 61,
        sounds: vec!["sfx/missing".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    assert!(manager.pending_one_shots.contains_key("sfx/missing"));
    assert!(manager.pending_loops.contains_key(&61));
    assert!(manager.pending_loads.contains_key("sfx/missing"));

    manager.fail_load_for_test("sfx/missing", "synthetic failure");
    manager.poll(0.0);

    assert!(!manager.pending_one_shots.contains_key("sfx/missing"));
    assert!(!manager.pending_loops.contains_key(&61));
    assert!(!manager.pending_loads.contains_key("sfx/missing"));
    assert!(manager.test_state.started_one_shot_playbacks.is_empty());
    assert!(manager.test_state.started_loop_playbacks.is_empty());
    assert!(manager.active_loops.is_empty());
}

#[test]
fn newer_pending_loop_request_replaces_older_request_for_same_handle() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayLoop {
        handle: 43,
        sounds: vec!["loop/first".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    push_audio_command(AudioCommand::PlayLoop {
        handle: 43,
        sounds: vec!["loop/second".to_string()],
        volume: 0.75,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    assert_eq!(
        manager
            .pending_loops
            .get(&43)
            .map(|pending| pending.sound_id.as_str()),
        Some("loop/second")
    );
    assert_eq!(manager.test_state.active_loop_sound_ids.get(&43), None);
    assert!(manager.test_state.started_loop_playbacks.is_empty());

    let pending = manager.pending_loops.get(&43).unwrap();
    let expected_volume = pending.volume;
    let expected_pitch = pending.pitch;

    manager.complete_load_for_test("loop/first", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert_eq!(manager.test_state.active_loop_sound_ids.get(&43), None);
    assert!(manager.test_state.started_loop_playbacks.is_empty());

    manager.complete_load_for_test("loop/second", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert_eq!(
        manager
            .test_state
            .active_loop_sound_ids
            .get(&43)
            .map(String::as_str),
        Some("loop/second")
    );
    assert_eq!(
        manager.test_state.started_loop_playbacks.get(&43),
        Some(&StartedLoopPlayback {
            id: "loop/second".to_string(),
            volume: expected_volume,
            pitch: expected_pitch,
        })
    );
}

#[test]
fn failed_cold_loop_load_clears_pending_state_without_playback() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayLoop {
        handle: 51,
        sounds: vec!["loop/fail".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    assert!(manager.pending_loops.contains_key(&51));
    assert!(manager.pending_loads.contains_key("loop/fail"));

    manager.fail_load_for_test("loop/fail", "boom");
    manager.poll(0.0);

    assert!(!manager.pending_loops.contains_key(&51));
    assert!(!manager.pending_loads.contains_key("loop/fail"));
    assert!(!manager.active_loops.contains_key(&51));
    assert!(!manager.test_state.active_loop_sound_ids.contains_key(&51));
}

#[cfg(feature = "editor")]
#[test]
fn tracked_preview_replaces_existing_preview_for_same_handle() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 11,
        sounds: vec!["preview/click".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 3.0,
    });
    manager.poll(0.0);
    assert_eq!(manager.tracked_previews.len(), 1);
    let first_expiry = manager
        .tracked_previews
        .get(&11)
        .map(|preview| preview.expires_at)
        .unwrap();

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 11,
        sounds: vec!["preview/click".to_string()],
        volume: 0.75,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 5.0,
    });
    manager.poll(0.0);

    assert_eq!(manager.tracked_previews.len(), 1);
    let second_expiry = manager
        .tracked_previews
        .get(&11)
        .map(|preview| preview.expires_at)
        .unwrap();
    assert!(second_expiry >= first_expiry + 1.0);
}

#[cfg(feature = "editor")]
#[test]
fn tracked_preview_expires_when_timeout_elapses() {
    let _ = drain_audio_commands();
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 17,
        sounds: vec!["preview/click".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 1.0,
    });
    manager.poll(0.5);
    assert!(manager.tracked_previews.contains_key(&17));

    manager.poll(0.5);
    assert!(manager.tracked_previews.contains_key(&17));

    manager.poll(0.5);
    assert!(!manager.tracked_previews.contains_key(&17));
}

#[cfg(feature = "editor")]
#[test]
fn tracked_preview_timeout_starts_after_the_preview_is_created() {
    let _ = drain_audio_commands();
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 19,
        sounds: vec!["preview/click".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 0.25,
    });
    manager.poll(1.0);

    assert!(manager.tracked_previews.contains_key(&19));

    manager.poll(0.25);
    assert!(!manager.tracked_previews.contains_key(&19));
}

#[cfg(feature = "editor")]
#[test]
fn uncached_tracked_preview_waits_for_load_before_starting() {
    let _ = drain_audio_commands();
    let mut manager = seeded_manager();
    manager.sound_cache.remove("preview/click");

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 27,
        sounds: vec!["preview/click".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 0.5,
    });
    manager.poll(0.0);

    assert!(manager.pending_loads.contains_key("preview/click"));
    assert!(manager.pending_previews.contains_key(&27));
    assert!(!manager.tracked_previews.contains_key(&27));

    manager.complete_load_for_test("preview/click", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(!manager.pending_previews.contains_key(&27));
    assert!(manager.tracked_previews.contains_key(&27));
    assert_eq!(
        manager
            .test_state
            .started_tracked_preview_playbacks
            .get(&27),
        Some(&StartedTrackedPreviewPlayback {
            id: "preview/click".to_string(),
            volume: 0.5,
            pitch: 1.0,
            looping: true,
        })
    );
}

#[cfg(feature = "editor")]
#[test]
fn failed_load_clears_pending_preview_requests() {
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 63,
        sounds: vec!["preview/missing".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 1.0,
    });
    manager.poll(0.0);

    assert!(manager.pending_previews.contains_key(&63));
    assert!(manager.pending_loads.contains_key("preview/missing"));
    assert!(!manager.tracked_previews.contains_key(&63));

    manager.fail_load_for_test("preview/missing", "synthetic failure");
    manager.poll(0.0);

    assert!(!manager.pending_previews.contains_key(&63));
    assert!(!manager.tracked_previews.contains_key(&63));
    assert!(!manager.pending_loads.contains_key("preview/missing"));
    assert!(
        manager
            .test_state
            .started_tracked_preview_playbacks
            .is_empty()
    );
}

#[cfg(feature = "editor")]
#[test]
fn stop_tracked_preview_cancels_pending_cold_preview() {
    let _ = drain_audio_commands();
    let mut manager = seeded_manager();
    manager.sound_cache.remove("preview/click");

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 29,
        sounds: vec!["preview/click".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: false,
        timeout: 0.5,
    });
    manager.poll(0.0);

    assert!(manager.pending_loads.contains_key("preview/click"));
    assert!(manager.pending_previews.contains_key(&29));

    push_audio_command(AudioCommand::StopTrackedPreview(29));
    manager.poll(0.0);

    assert!(!manager.pending_previews.contains_key(&29));

    manager.complete_load_for_test("preview/click", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(!manager.tracked_previews.contains_key(&29));
}

#[cfg(feature = "editor")]
#[test]
fn cold_preview_timeout_starts_when_playback_begins() {
    let _ = drain_audio_commands();
    let mut manager = seeded_manager();
    manager.sound_cache.remove("preview/click");

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 31,
        sounds: vec!["preview/click".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 0.25,
    });
    manager.poll(1.0);

    assert!(manager.pending_loads.contains_key("preview/click"));
    assert!(manager.pending_previews.contains_key(&31));
    assert!(!manager.tracked_previews.contains_key(&31));

    manager.complete_load_for_test("preview/click", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(manager.tracked_previews.contains_key(&31));
    let expires_at = manager
        .tracked_previews
        .get(&31)
        .map(|preview| preview.expires_at)
        .unwrap();
    assert_approx_eq(expires_at, manager.preview_time + 0.25);

    manager.poll(0.24);
    assert!(manager.tracked_previews.contains_key(&31));

    manager.poll(0.01);
    assert!(!manager.tracked_previews.contains_key(&31));
}

#[cfg(feature = "editor")]
#[test]
fn cold_tracked_preview_reuses_captured_request_time_selection_and_variation() {
    let _ = drain_audio_commands();
    let mut manager = seeded_manager();
    manager.sound_cache.remove("preview/click");
    manager.sound_cache.remove("preview/alt");

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 33,
        sounds: vec!["preview/click".to_string(), "preview/alt".to_string()],
        volume: 0.5,
        pitch_variation: 0.25,
        volume_variation: 0.25,
        looping: false,
        timeout: 0.5,
    });
    manager.poll(0.0);

    assert_eq!(manager.pending_previews.len(), 1);
    let pending = manager.pending_previews.get(&33).unwrap();
    assert!(matches!(
        pending.sound_id.as_str(),
        "preview/click" | "preview/alt"
    ));
    assert!(manager.pending_loads.contains_key(&pending.sound_id));

    let pending_sound_id = pending.sound_id.clone();
    let pending_volume = pending.volume;
    let pending_pitch = pending.pitch;
    let pending_looping = pending.looping;

    manager.complete_load_for_test(&pending_sound_id, Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(!manager.pending_previews.contains_key(&33));
    assert_eq!(
        manager
            .test_state
            .started_tracked_preview_playbacks
            .get(&33),
        Some(&StartedTrackedPreviewPlayback {
            id: pending_sound_id,
            volume: pending_volume,
            pitch: pending_pitch,
            looping: pending_looping,
        })
    );
}

#[cfg(feature = "editor")]
#[test]
fn latest_cold_tracked_preview_wins_for_same_handle_before_loads_finish() {
    let _ = drain_audio_commands();
    let mut manager = seeded_manager();
    manager.sound_cache.remove("preview/first");
    manager.sound_cache.remove("preview/second");

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 35,
        sounds: vec!["preview/first".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 0.5,
    });
    manager.poll(0.0);

    assert_eq!(
        manager
            .pending_previews
            .get(&35)
            .map(|pending| pending.sound_id.as_str()),
        Some("preview/first")
    );
    assert!(manager.pending_loads.contains_key("preview/first"));

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 35,
        sounds: vec!["preview/second".to_string()],
        volume: 0.75,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 1.0,
    });
    manager.poll(0.0);

    assert_eq!(manager.pending_previews.len(), 1);
    assert_eq!(
        manager
            .pending_previews
            .get(&35)
            .map(|pending| pending.sound_id.as_str()),
        Some("preview/second")
    );
    assert!(manager.pending_loads.contains_key("preview/first"));
    assert!(manager.pending_loads.contains_key("preview/second"));

    manager.complete_load_for_test("preview/first", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(manager.pending_previews.contains_key(&35));
    assert!(!manager.tracked_previews.contains_key(&35));
    assert!(
        !manager
            .test_state
            .started_tracked_preview_playbacks
            .contains_key(&35)
    );

    manager.complete_load_for_test("preview/second", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(!manager.pending_previews.contains_key(&35));
    assert_eq!(
        manager
            .test_state
            .started_tracked_preview_playbacks
            .get(&35),
        Some(&StartedTrackedPreviewPlayback {
            id: "preview/second".to_string(),
            volume: 0.75,
            pitch: 1.0,
            looping: true,
        })
    );
    assert!(manager.tracked_previews.contains_key(&35));
}

#[cfg(feature = "editor")]
#[test]
fn stop_tracked_one_shot_preview_removes_preview_handle() {
    let _ = drain_audio_commands();
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 21,
        sounds: vec!["preview/click".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: false,
        timeout: 5.0,
    });
    manager.poll(0.0);
    assert!(manager.tracked_previews.contains_key(&21));

    push_audio_command(AudioCommand::StopTrackedPreview(21));
    manager.poll(0.0);
    assert!(!manager.tracked_previews.contains_key(&21));
}

#[cfg(feature = "editor")]
#[test]
fn stop_tracked_preview_removes_preview_handle() {
    let _ = drain_audio_commands();
    let mut manager = seeded_manager();

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 23,
        sounds: vec!["preview/click".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        timeout: 5.0,
    });
    manager.poll(0.0);
    assert!(manager.tracked_previews.contains_key(&23));

    push_audio_command(AudioCommand::StopTrackedPreview(23));
    manager.poll(0.0);
    assert!(!manager.tracked_previews.contains_key(&23));
}
