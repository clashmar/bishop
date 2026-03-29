use super::*;
use crate::audio::audio_source::{
    AudioGroup, AudioSource, SoundGroupId, SoundPresetLink, test_post_create, test_post_remove,
};
use crate::audio::command_queue::{PlayMusicRequest, drain_audio_commands, push_audio_command};
use crate::ecs::entity::Entity;
use crate::game::Game;
use serde::Deserialize;

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
