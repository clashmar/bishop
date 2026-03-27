use super::*;
use crate::audio::audio_source::{
    AudioGroup, AudioSource, SoundGroupId, SoundPresetLink, test_post_create, test_post_remove,
};
use crate::audio::command_queue::{PlayMusicRequest, drain_audio_commands, push_audio_command};
use crate::audio::runtime;
use crate::ecs::entity::Entity;
use crate::game::Game;
use crate::task::BackgroundService;
use bishop::audio::AudioBackend;
use oddio::Frames;
use serde::Deserialize;

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
fn play_music_request_can_be_queued_and_drained() {
    let _ = drain_audio_commands();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: false,
        fade_out: 0.5,
        gap: 0.25,
        fade_in: 0.75,
    }));

    let commands = drain_audio_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        AudioCommand::PlayMusic(request) => {
            assert_eq!(request.id, "music/intro");
            assert!(!request.looping);
            assert_eq!(request.fade_out, 0.5);
            assert_eq!(request.gap, 0.25);
            assert_eq!(request.fade_in, 0.75);
        }
        _ => panic!("expected PlayMusic"),
    }
}

#[cfg(feature = "editor")]
#[test]
fn tracked_preview_commands_can_be_queued_and_drained() {
    let _ = drain_audio_commands();

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 7,
        sounds: vec!["ui/click".to_string()],
        volume: 0.75,
        pitch_variation: 0.1,
        volume_variation: 0.2,
        looping: true,
        timeout: 1.5,
    });
    push_audio_command(AudioCommand::StopTrackedPreview(7));

    let commands = drain_audio_commands();
    assert_eq!(commands.len(), 2);
    match &commands[0] {
        AudioCommand::PlayTrackedPreview {
            handle,
            sounds,
            volume,
            pitch_variation,
            volume_variation,
            looping,
            timeout,
        } => {
            assert_eq!(*handle, 7);
            assert_eq!(sounds, &vec!["ui/click".to_string()]);
            assert_eq!(*volume, 0.75);
            assert_eq!(*pitch_variation, 0.1);
            assert_eq!(*volume_variation, 0.2);
            assert!(*looping);
            assert_eq!(*timeout, 1.5);
        }
        _ => panic!("expected PlayTrackedPreview"),
    }
    match &commands[1] {
        AudioCommand::StopTrackedPreview(handle) => assert_eq!(*handle, 7),
        _ => panic!("expected StopTrackedPreview"),
    }
}

#[test]
fn sound_group_id_ui_label_uses_custom_name() {
    assert_eq!(
        SoundGroupId::Custom("Footsteps".to_string()).ui_label(),
        "Footsteps"
    );
    assert_eq!(SoundGroupId::New.ui_label(), "Add Group");
}

#[test]
fn audio_group_defaults_to_full_volume() {
    assert_eq!(AudioGroup::default().volume, 1.0);
}

#[test]
fn all_sound_ids_collects_every_group_sound() {
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::Custom("Footsteps".to_string()),
        AudioGroup {
            sounds: vec!["footstep_a".to_string(), "footstep_b".to_string()],
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: vec!["talk_a".to_string()],
            ..Default::default()
        },
    );

    let mut ids = source.all_sound_ids();
    ids.sort();

    assert_eq!(
        ids,
        vec![
            "footstep_a".to_string(),
            "footstep_b".to_string(),
            "talk_a".to_string(),
        ]
    );
}

#[test]
fn all_sound_ids_deduplicates_repeated_sound_ids() {
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::Custom("One".to_string()),
        AudioGroup {
            sounds: vec!["shared".to_string(), "shared".to_string()],
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Two".to_string()),
        AudioGroup {
            sounds: vec!["shared".to_string(), "unique".to_string()],
            ..Default::default()
        },
    );

    assert_eq!(
        source.all_sound_ids(),
        vec!["shared".to_string(), "unique".to_string()]
    );
}

#[test]
fn apply_preset_to_linked_group_overwrites_local_fields() {
    let preset = AudioGroup {
        sounds: vec!["talk_a".to_string()],
        volume: 0.5,
        pitch_variation: 0.1,
        volume_variation: 0.2,
        looping: false,
        preset_link: None,
    };

    let mut group = AudioGroup {
        sounds: vec!["old".to_string()],
        volume: 1.0,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        preset_link: Some(SoundPresetLink {
            preset_name: "OldPreset".to_string(),
        }),
    };

    group.apply_preset("Talk", &preset);

    assert_eq!(group.sounds, vec!["talk_a".to_string()]);
    assert_eq!(group.volume, 0.5);
    assert_eq!(group.pitch_variation, 0.1);
    assert_eq!(group.volume_variation, 0.2);
    assert!(!group.looping);
    assert_eq!(
        group.preset_link,
        Some(SoundPresetLink {
            preset_name: "Talk".to_string(),
        })
    );
}

#[test]
fn all_sound_ids_ignores_new_group() {
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::New,
        AudioGroup {
            sounds: vec!["temp".to_string()],
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: vec!["talk_1".to_string()],
            ..Default::default()
        },
    );

    assert_eq!(source.all_sound_ids(), vec!["talk_1".to_string()]);
}

#[test]
fn deserializing_grouped_audio_source_preserves_groups() {
    #[derive(Deserialize)]
    struct Wrapper {
        source: AudioSource,
    }

    let ron = r#"
        (
            source: (
                groups: {
                    Custom("Talk"): (
                        sounds: ["talk_1", "talk_2"],
                        volume: 0.8,
                        pitch_variation: 0.1,
                        volume_variation: 0.2,
                        looping: false,
                    ),
                },
            ),
        )
    "#;

    let wrapper: Wrapper = ron::from_str(ron).unwrap();
    let group = wrapper
        .source
        .groups
        .get(&SoundGroupId::Custom("Talk".to_string()))
        .unwrap();

    assert_eq!(
        group.sounds,
        vec!["talk_1".to_string(), "talk_2".to_string()]
    );
    assert_eq!(group.volume, 0.8);
    assert_eq!(group.pitch_variation, 0.1);
    assert_eq!(group.volume_variation, 0.2);
    assert!(!group.looping);
    assert!(group.preset_link.is_none());
    assert!(wrapper.source.current.is_none());
}

#[test]
fn deserializing_group_without_volume_uses_full_volume_default() {
    #[derive(Deserialize)]
    struct Wrapper {
        source: AudioSource,
    }

    let ron = r#"
        (
            source: (
                groups: {
                    Custom("Talk"): (
                        sounds: ["talk_1"],
                    ),
                },
            ),
        )
    "#;

    let wrapper: Wrapper = ron::from_str(ron).unwrap();
    let group = wrapper
        .source
        .groups
        .get(&SoundGroupId::Custom("Talk".to_string()))
        .unwrap();

    assert_eq!(group.volume, 1.0);
}

#[test]
fn deserializing_negative_variations_clamps_to_zero() {
    #[derive(Deserialize)]
    struct Wrapper {
        source: AudioSource,
    }

    let ron = r#"
        (
            source: (
                groups: {
                    Custom("Talk"): (
                        sounds: ["talk_1"],
                        pitch_variation: -0.25,
                        volume_variation: -0.5,
                    ),
                },
            ),
        )
    "#;

    let wrapper: Wrapper = ron::from_str(ron).unwrap();
    let group = wrapper
        .source
        .groups
        .get(&SoundGroupId::Custom("Talk".to_string()))
        .unwrap();

    assert_eq!(group.pitch_variation, 0.0);
    assert_eq!(group.volume_variation, 0.0);
}

#[test]
fn serializing_audio_source_omits_new_group_keys() {
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::New,
        AudioGroup {
            sounds: vec!["temp".to_string()],
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: vec!["talk_1".to_string()],
            ..Default::default()
        },
    );

    let ron = ron::to_string(&source).unwrap();

    assert!(!ron.contains("New"));
    assert!(ron.contains(r#"Custom("Talk")"#));
    assert!(ron.contains(r#"sounds:["talk_1"]"#));
}

#[test]
fn serializing_audio_source_orders_groups_deterministically() {
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::Custom("Zulu".to_string()),
        AudioGroup {
            sounds: vec!["z".to_string()],
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Alpha".to_string()),
        AudioGroup {
            sounds: vec!["a".to_string()],
            ..Default::default()
        },
    );

    let ron = ron::to_string(&source).unwrap();

    let alpha_index = ron.find(r#"Custom("Alpha")"#).unwrap();
    let zulu_index = ron.find(r#"Custom("Zulu")"#).unwrap();
    assert!(alpha_index < zulu_index);
}

#[test]
fn serializing_audio_source_round_trips_structurally() {
    let mut source = AudioSource {
        current: Some(SoundGroupId::Custom("Talk".to_string())),
        ..Default::default()
    };
    source.groups.insert(
        SoundGroupId::New,
        AudioGroup {
            sounds: vec!["temp".to_string()],
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: vec!["talk_1".to_string()],
            volume: 0.75,
            ..Default::default()
        },
    );

    let ron = ron::to_string(&source).unwrap();
    let round_trip: AudioSource = ron::from_str(&ron).unwrap();

    assert!(round_trip.current.is_none());
    assert_eq!(round_trip.groups.len(), 1);
    assert_eq!(
        round_trip
            .groups
            .get(&SoundGroupId::Custom("Talk".to_string()))
            .unwrap(),
        &AudioGroup {
            sounds: vec!["talk_1".to_string()],
            volume: 0.75,
            pitch_variation: 0.0,
            volume_variation: 0.0,
            looping: false,
            preset_link: None,
        }
    );
}

#[test]
fn deserializing_audio_source_drops_new_group_key() {
    #[derive(Deserialize)]
    struct Wrapper {
        source: AudioSource,
    }

    let ron = r#"
        (
            source: (
                groups: {
                    New: (
                        sounds: ["temp"],
                    ),
                    Custom("Talk"): (
                        sounds: ["talk_1"],
                    ),
                },
            ),
        )
    "#;

    let wrapper: Wrapper = ron::from_str(ron).unwrap();

    assert!(!wrapper.source.groups.contains_key(&SoundGroupId::New));
    assert!(
        wrapper
            .source
            .groups
            .contains_key(&SoundGroupId::Custom("Talk".to_string()))
    );
}

#[test]
fn deserializing_audio_source_rejects_unknown_fields() {
    let ron = r#"
        (
            groups: {
                Custom("Talk"): (
                    sounds: ["talk_1"],
                    unexpected: true,
                ),
            },
        )
    "#;

    let result: Result<AudioSource, _> = ron::from_str(ron);
    assert!(result.is_err());
}

#[test]
fn post_create_ignores_new_group_when_incrementing_refs() {
    let _ = drain_audio_commands();

    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::New,
        AudioGroup {
            sounds: vec!["temp".to_string()],
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: vec!["talk_1".to_string(), "talk_1".to_string()],
            ..Default::default()
        },
    );

    let mut game = Game::default();
    game.worlds.push(Default::default());
    let mut ctx = game.ctx_mut();

    test_post_create(&mut source, &Entity(7), &mut ctx);

    let commands = drain_audio_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        AudioCommand::IncrementRefs(ids) => {
            assert_eq!(ids, &vec!["talk_1".to_string()]);
        }
        _ => panic!("expected IncrementRefs"),
    }
}

#[test]
fn post_remove_ignores_new_group_when_decrementing_refs() {
    let _ = drain_audio_commands();

    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::New,
        AudioGroup {
            sounds: vec!["temp".to_string()],
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: vec!["talk_1".to_string()],
            ..Default::default()
        },
    );

    let entity = Entity(9);
    let mut game = Game::default();
    game.worlds.push(Default::default());
    let mut ctx = game.ctx_mut();

    test_post_remove(&mut source, &entity, &mut ctx);

    let commands = drain_audio_commands();
    assert_eq!(commands.len(), 2);
    match &commands[0] {
        AudioCommand::StopLoop(handle) => assert_eq!(*handle, 9),
        _ => panic!("expected StopLoop"),
    }
    match &commands[1] {
        AudioCommand::DecrementRefs(ids) => {
            assert_eq!(ids, &vec!["talk_1".to_string()]);
        }
        _ => panic!("expected DecrementRefs"),
    }
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
