use super::{
    assert_approx_eq, push_audio_command, runtime, seeded_manager, AudioCommand,
    BackgroundService, MusicTransition, PlayMusicRequest,
};

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

