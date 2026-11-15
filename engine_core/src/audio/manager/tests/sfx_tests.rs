use super::{
    drain_audio_commands, entity_loop_key, push_audio_command, runtime, seeded_manager,
    AudioCommand, AudioLoopKey, AudioManager, AudioPlaybackOwner, BackgroundService, Entity,
    Frames, Hydratable, PendingOneShot, RoomId, SoundGroupId, StartedLoopPlayback,
    StartedOneShotPlayback, TestBackend, WorldId,
};

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
fn stop_loops_with_fade_keeps_sound_referenced_until_fade_completes() {
    runtime::reset_for_tests();
    let _ = drain_audio_commands();
    let mut manager = AudioManager::new::<TestBackend>();
    manager.complete_load_for_test("sfx/wind", Frames::from_slice(44_100, &[[0.0, 0.0]]));

    push_audio_command(AudioCommand::PlayLoop {
        key: AudioLoopKey::new(
            AudioPlaybackOwner::Room(RoomId(4)),
            SoundGroupId::Custom("Ambience".to_string()),
        ),
        sounds: vec!["sfx/wind".to_string()],
        volume: 1.0,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    push_audio_command(AudioCommand::StopLoops {
        owner: AudioPlaybackOwner::Room(RoomId(4)),
        fade_out: Some(0.5),
    });
    manager.poll(0.1);
    assert_eq!(manager.ref_count(&"sfx/wind".to_string()), 1);

    manager.poll(0.5);
    assert_eq!(manager.ref_count(&"sfx/wind".to_string()), 0);
}

#[test]
fn stop_loops_immediately_releases_owner_playback() {
    runtime::reset_for_tests();
    let _ = drain_audio_commands();
    let mut manager = AudioManager::new::<TestBackend>();
    manager.complete_load_for_test("sfx/hum", Frames::from_slice(44_100, &[[0.0, 0.0]]));

    push_audio_command(AudioCommand::PlayLoop {
        key: AudioLoopKey::new(
            AudioPlaybackOwner::World(WorldId(2)),
            SoundGroupId::Custom("Hum".to_string()),
        ),
        sounds: vec!["sfx/hum".to_string()],
        volume: 1.0,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    push_audio_command(AudioCommand::StopLoops {
        owner: AudioPlaybackOwner::World(WorldId(2)),
        fade_out: None,
    });
    manager.poll(0.0);

    assert_eq!(manager.ref_count(&"sfx/hum".to_string()), 0);
}

#[test]
fn stop_loops_cancel_pending_cold_loop_before_playback_starts() {
    let mut manager = seeded_manager();
    let key = entity_loop_key(41, "ColdLoop");

    push_audio_command(AudioCommand::PlayLoop {
        key: key.clone(),
        sounds: vec!["loop/cold".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    assert!(manager.pending_loops.contains_key(&key));
    assert!(manager.active_loops.is_empty());

    push_audio_command(AudioCommand::StopLoops {
        owner: AudioPlaybackOwner::Entity(Entity(41)),
        fade_out: None,
    });
    manager.poll(0.0);

    assert!(!manager.pending_loops.contains_key(&key));

    manager.complete_load_for_test("loop/cold", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert!(!manager.active_loops.contains_key(&key));
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
    let key = entity_loop_key(61, "MissingLoop");

    push_audio_command(AudioCommand::PlaySfx("sfx/missing".to_string()));
    push_audio_command(AudioCommand::PlayLoop {
        key: key.clone(),
        sounds: vec!["sfx/missing".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    assert!(manager.pending_one_shots.contains_key("sfx/missing"));
    assert!(manager.pending_loops.contains_key(&key));
    assert!(manager.pending_loads.contains_key("sfx/missing"));

    manager.fail_load_for_test("sfx/missing", "synthetic failure");
    manager.poll(0.0);

    assert!(!manager.pending_one_shots.contains_key("sfx/missing"));
    assert!(!manager.pending_loops.contains_key(&key));
    assert!(!manager.pending_loads.contains_key("sfx/missing"));
    assert!(manager.test_state.started_one_shot_playbacks.is_empty());
    assert!(manager.test_state.started_loop_playbacks.is_empty());
    assert!(manager.active_loops.is_empty());
}

#[test]
fn newer_pending_loop_request_replaces_older_request_for_same_key() {
    let mut manager = seeded_manager();
    let key = entity_loop_key(43, "ScopedLoop");

    push_audio_command(AudioCommand::PlayLoop {
        key: key.clone(),
        sounds: vec!["loop/first".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    push_audio_command(AudioCommand::PlayLoop {
        key: key.clone(),
        sounds: vec!["loop/second".to_string()],
        volume: 0.75,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    assert_eq!(
        manager
            .pending_loops
            .get(&key)
            .map(|pending| pending.sound_id.as_str()),
        Some("loop/second")
    );
    assert_eq!(manager.test_state.active_loop_sound_ids.get(&key), None);
    assert!(manager.test_state.started_loop_playbacks.is_empty());

    let pending = manager.pending_loops.get(&key).unwrap();
    let expected_volume = pending.volume;
    let expected_pitch = pending.pitch;

    manager.complete_load_for_test("loop/first", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert_eq!(manager.test_state.active_loop_sound_ids.get(&key), None);
    assert!(manager.test_state.started_loop_playbacks.is_empty());

    manager.complete_load_for_test("loop/second", Frames::from_slice(44_100, &[[0.0, 0.0]]));
    manager.poll(0.0);

    assert_eq!(
        manager
            .test_state
            .active_loop_sound_ids
            .get(&key)
            .map(String::as_str),
        Some("loop/second")
    );
    assert_eq!(
        manager.test_state.started_loop_playbacks.get(&key),
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
    let key = entity_loop_key(51, "FailLoop");

    push_audio_command(AudioCommand::PlayLoop {
        key: key.clone(),
        sounds: vec!["loop/fail".to_string()],
        volume: 0.5,
        pitch_variation: 0.0,
        volume_variation: 0.0,
    });
    manager.poll(0.0);

    assert!(manager.pending_loops.contains_key(&key));
    assert!(manager.pending_loads.contains_key("loop/fail"));

    manager.fail_load_for_test("loop/fail", "boom");
    manager.poll(0.0);

    assert!(!manager.pending_loops.contains_key(&key));
    assert!(!manager.pending_loads.contains_key("loop/fail"));
    assert!(!manager.active_loops.contains_key(&key));
    assert!(!manager.test_state.active_loop_sound_ids.contains_key(&key));
}

