use super::{
    assert_approx_eq, drain_audio_commands, push_audio_command, seeded_manager, AudioCommand,
    BackgroundService, Frames, StartedTrackedPreviewPlayback,
};

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
    assert!(manager
        .test_state
        .started_tracked_preview_playbacks
        .is_empty());
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
    assert!(!manager
        .test_state
        .started_tracked_preview_playbacks
        .contains_key(&35));

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