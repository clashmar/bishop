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
    assert_eq!(snapshot.pinned_sound_count, 0);
    assert_eq!(snapshot.ref_count_entry_count, 0);
    assert_eq!(snapshot.entries.len(), 3);
    assert_eq!(
        snapshot.entries,
        vec![
            AudioDiagnosticsEntry {
                id: "music/intro".to_string(),
                cached: true,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "music/next".to_string(),
                cached: true,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "preview/click".to_string(),
                cached: true,
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
    assert_eq!(snapshot.pinned_sound_count, 1);
    assert_eq!(snapshot.ref_count_entry_count, 2);
    assert_eq!(snapshot.entries.len(), 6);
    assert_eq!(
        snapshot.entries,
        vec![
            AudioDiagnosticsEntry {
                id: "music/intro".to_string(),
                cached: true,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "music/next".to_string(),
                cached: true,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "pinned/only".to_string(),
                cached: false,
                pinned: true,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "preview/click".to_string(),
                cached: true,
                pinned: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "ref/only".to_string(),
                cached: false,
                pinned: false,
                ref_count: 2,
            },
            AudioDiagnosticsEntry {
                id: "shared".to_string(),
                cached: true,
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
