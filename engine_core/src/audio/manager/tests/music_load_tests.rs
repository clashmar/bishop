use super::{
    push_audio_command, runtime, seeded_manager, ActiveMusic, AudioCommand, BackgroundService,
    Frames, MusicTransition, PlayMusicRequest,
};

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

