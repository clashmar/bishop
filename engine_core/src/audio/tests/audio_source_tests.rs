use crate::audio::command_queue::drain_audio_commands;
use crate::audio::{AudioCommand, AudioPlaybackOwner};
use crate::constants::paths::SFX_FOLDER;
use crate::ecs::components::audio_source::test_post_remove;
use crate::ecs::entity::Entity;
use crate::ecs::{
    AudioGroup, AudioSource, AudioStopBehavior, AudioTrigger, SoundGroupId, SoundId,
    SoundPresetLink,
};
use crate::game::Game;
use crate::worlds::World;
use serde::Deserialize;
use std::collections::HashMap;

fn sound_ids(ids: &[usize]) -> Vec<SoundId> {
    ids.iter().copied().map(SoundId).collect()
}

#[test]
fn all_sound_ids_collects_every_group_sound() {
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::Custom("Footsteps".to_string()),
        AudioGroup {
            sounds: sound_ids(&[1, 2]),
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: sound_ids(&[3]),
            ..Default::default()
        },
    );

    let mut ids = source.all_sound_ids();
    ids.sort();

    assert_eq!(ids, sound_ids(&[1, 2, 3]));
}

#[test]
fn all_sound_ids_deduplicates_repeated_sound_ids() {
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::Custom("One".to_string()),
        AudioGroup {
            sounds: sound_ids(&[1, 1]),
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Two".to_string()),
        AudioGroup {
            sounds: sound_ids(&[1, 2]),
            ..Default::default()
        },
    );

    assert_eq!(source.all_sound_ids(), sound_ids(&[1, 2]));
}

#[test]
fn apply_preset_to_linked_group_overwrites_local_fields() {
    let preset = AudioGroup {
        sounds: sound_ids(&[3]),
        volume: 0.5,
        pitch_variation: 0.1,
        volume_variation: 0.2,
        looping: false,
        preset_link: None,
        ..Default::default()
    };

    let mut group = AudioGroup {
        sounds: sound_ids(&[9]),
        volume: 1.0,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        preset_link: Some(SoundPresetLink {
            preset_name: "OldPreset".to_string(),
        }),
        ..Default::default()
    };

    group.apply_preset("Talk", &preset);

    assert_eq!(group.sounds, sound_ids(&[3]));
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
            sounds: sound_ids(&[99]),
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: sound_ids(&[4]),
            ..Default::default()
        },
    );

    assert_eq!(source.all_sound_ids(), sound_ids(&[4]));
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
                        sounds: [SoundId(1), SoundId(2)],
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

    assert_eq!(group.sounds, sound_ids(&[1, 2]));
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
                        sounds: [SoundId(1)],
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
                        sounds: [SoundId(1)],
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
            sounds: sound_ids(&[99]),
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: sound_ids(&[1]),
            ..Default::default()
        },
    );

    let ron = ron::to_string(&source).unwrap();

    assert!(!ron.contains("New"));
    assert!(ron.contains(r#"Custom("Talk")"#));
    assert!(ron.contains("sounds:["));
    assert!(!ron.contains("99"));
    assert!(!ron.contains("talk_1"));
}

#[test]
fn serializing_audio_source_orders_groups_deterministically() {
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::Custom("Zulu".to_string()),
        AudioGroup {
            sounds: sound_ids(&[26]),
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Alpha".to_string()),
        AudioGroup {
            sounds: sound_ids(&[1]),
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
            sounds: sound_ids(&[99]),
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: sound_ids(&[11]),
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
            sounds: sound_ids(&[11]),
            volume: 0.75,
            pitch_variation: 0.0,
            volume_variation: 0.0,
            looping: false,
            preset_link: None,
            ..Default::default()
        }
    );
}

#[test]
fn audio_group_defaults_to_manual_trigger_and_immediate_stop() {
    let group = AudioGroup::default();

    assert_eq!(group.trigger, AudioTrigger::Manual);
    assert_eq!(group.stop_behavior, AudioStopBehavior::Immediate);
}

#[test]
fn audio_group_round_trips_trigger_and_fade_stop_behavior() {
    let source = AudioSource {
        groups: HashMap::from([(
            SoundGroupId::Custom("Ambience".to_string()),
            AudioGroup {
                sounds: sound_ids(&[7]),
                looping: true,
                trigger: AudioTrigger::OnOwnerActivate,
                stop_behavior: AudioStopBehavior::FadeOut { duration: 0.5 },
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    let ron = ron::ser::to_string(&source).unwrap();
    let parsed: AudioSource = ron::from_str(&ron).unwrap();
    let group = parsed
        .groups
        .get(&SoundGroupId::Custom("Ambience".to_string()))
        .unwrap();

    assert_eq!(group.trigger, AudioTrigger::OnOwnerActivate);
    assert_eq!(
        group.stop_behavior,
        AudioStopBehavior::FadeOut { duration: 0.5 }
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
                        sounds: [SoundId(99)],
                    ),
                    Custom("Talk"): (
                        sounds: [SoundId(1)],
                    ),
                },
            ),
        )
    "#;

    let wrapper: Wrapper = ron::from_str(ron).unwrap();

    assert!(!wrapper.source.groups.contains_key(&SoundGroupId::New));
    assert!(wrapper
        .source
        .groups
        .contains_key(&SoundGroupId::Custom("Talk".to_string())));
}

#[test]
fn deserializing_audio_source_rejects_unknown_fields() {
    let ron = r#"
        (
            groups: {
                Custom("Talk"): (
                    sounds: [SoundId(1)],
                    unexpected: true,
                ),
            },
        )
    "#;

    let result: Result<AudioSource, _> = ron::from_str(ron);
    assert!(result.is_err());
}

#[test]
fn post_remove_only_stops_loops() {
    let _ = drain_audio_commands();

    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::New,
        AudioGroup {
            sounds: sound_ids(&[99]),
            ..Default::default()
        },
    );
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup {
            sounds: sound_ids(&[1]),
            ..Default::default()
        },
    );

    let entity = Entity(9);
    let mut game = Game::default();
    game.add_world(World::default());
    game.asset_registry
        .register_asset_relative_path(
            SoundId(1),
            std::path::PathBuf::from(SFX_FOLDER).join("talk_1.wav"),
        )
        .unwrap();
    let mut ctx = game.ctx_mut();

    test_post_remove(&mut source, &entity, &mut ctx);

    let commands = drain_audio_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        AudioCommand::StopLoops { owner, fade_out } => {
            assert_eq!(owner, &AudioPlaybackOwner::Entity(Entity(9)));
            assert_eq!(*fade_out, None);
        }
        _ => panic!("expected StopLoops"),
    }
}
