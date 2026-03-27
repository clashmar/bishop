use crate::audio::command_queue::{AudioCommand, push_audio_command};
use crate::ecs::entity::Entity;
use crate::game::GameCtxMut;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde::ser::SerializeStruct;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Display, Formatter, Result as FmtResult};

/// Identifies a sound group stored on an [`AudioSource`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SoundGroupId {
    /// Creation-time placeholder for adding a new group in the editor.
    #[default]
    New,
    /// A user-defined group name.
    Custom(String),
}

impl SoundGroupId {
    /// Returns a label suitable for UI display.
    pub fn ui_label(&self) -> String {
        match self {
            Self::New => "Add Group".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
}

impl Display for SoundGroupId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.ui_label())
    }
}

/// Link to a shared sound preset stored by the editor.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundPresetLink {
    /// The preset name in the project-wide preset library.
    pub preset_name: String,
}

/// A single grouped audio definition attached to an entity.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(default)]
pub struct AudioGroup {
    /// Sound file paths relative to `Resources/audio/` without extension.
    pub sounds: Vec<String>,
    /// Base volume 0.0–1.0, defaulting to 1.0 and multiplied with the SFX group volume.
    pub volume: f32,
    /// Random pitch shift range: playback speed = 1.0 ± pitch_variation.
    pub pitch_variation: f32,
    /// Random volume jitter range: playback vol = volume ± volume_variation.
    pub volume_variation: f32,
    /// Whether this group loops continuously until stopped.
    pub looping: bool,
    /// Optional link to a shared preset.
    pub preset_link: Option<SoundPresetLink>,
}

fn default_audio_group_volume() -> f32 {
    1.0
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AudioGroupSerde {
    #[serde(default)]
    sounds: Vec<String>,
    #[serde(default = "default_audio_group_volume")]
    volume: f32,
    #[serde(default)]
    pitch_variation: f32,
    #[serde(default)]
    volume_variation: f32,
    #[serde(default)]
    looping: bool,
    #[serde(default)]
    preset_link: Option<SoundPresetLink>,
}

impl From<AudioGroupSerde> for AudioGroup {
    fn from(value: AudioGroupSerde) -> Self {
        let mut group = Self {
            sounds: value.sounds,
            volume: value.volume,
            pitch_variation: value.pitch_variation,
            volume_variation: value.volume_variation,
            looping: value.looping,
            preset_link: value.preset_link,
        };
        group.sanitize();
        group
    }
}

impl<'de> Deserialize<'de> for AudioGroup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        AudioGroupSerde::deserialize(deserializer).map(Into::into)
    }
}

impl AudioGroup {
    fn sanitize(&mut self) {
        self.pitch_variation = self.pitch_variation.max(0.0);
        self.volume_variation = self.volume_variation.max(0.0);
    }

    /// Overwrites the group's local settings from a preset and stores the active link.
    pub fn apply_preset(&mut self, preset_name: &str, preset: &AudioGroup) {
        self.sounds = preset.sounds.clone();
        self.volume = preset.volume;
        self.pitch_variation = preset.pitch_variation;
        self.volume_variation = preset.volume_variation;
        self.looping = preset.looping;
        self.preset_link = Some(SoundPresetLink {
            preset_name: preset_name.to_string(),
        });
        self.sanitize();
    }
}

impl Default for AudioGroup {
    fn default() -> Self {
        Self {
            sounds: Vec::new(),
            volume: default_audio_group_volume(),
            pitch_variation: 0.0,
            volume_variation: 0.0,
            looping: false,
            preset_link: None,
        }
    }
}

/// An ECS component that declares grouped audio clips an entity can play.
///
/// Sounds are organized into local groups so gameplay can reference
/// `entity:play_sound(sound.GroupName)`.
#[ecs_component(post_create = post_create, post_remove = post_remove)]
#[derive(Clone, Debug, PartialEq)]
pub struct AudioSource {
    /// Grouped sounds keyed by local group name.
    pub groups: HashMap<SoundGroupId, AudioGroup>,
    /// The currently selected group in the editor UI.
    pub current: Option<SoundGroupId>,
    /// Runtime gain multiplier applied on top of each group's authored volume.
    pub runtime_volume: f32,
}

impl AudioSource {
    /// Returns every sound ID referenced by every group.
    pub fn all_sound_ids(&self) -> Vec<String> {
        let mut ids = self
            .groups
            .iter()
            .filter(|(group_id, _)| !matches!(group_id, SoundGroupId::New))
            .flat_map(|(_, group)| group.sounds.iter().cloned())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            groups: HashMap::new(),
            current: None,
            runtime_volume: 1.0,
        }
    }
}

impl AudioSource {
    fn persisted_groups(&self) -> BTreeMap<&SoundGroupId, &AudioGroup> {
        self.groups
            .iter()
            .filter_map(|(group_id, group)| match group_id {
                SoundGroupId::New => None,
                _ => Some((group_id, group)),
            })
            .collect()
    }
}

impl Serialize for AudioSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let groups = self.persisted_groups();

        let mut state = serializer.serialize_struct("AudioSource", 1)?;
        state.serialize_field("groups", &groups)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for AudioSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut groups = AudioSourceSerde::deserialize(deserializer)?.groups;
        groups.retain(|group_id, group| {
            let keep = !matches!(group_id, SoundGroupId::New);
            if keep {
                group.sanitize();
            }
            keep
        });

        Ok(Self {
            groups,
            current: None,
            runtime_volume: 1.0,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AudioSourceSerde {
    #[serde(default)]
    groups: HashMap<SoundGroupId, AudioGroup>,
}

fn post_create(source: &mut AudioSource, _entity: &Entity, _ctx: &mut GameCtxMut) {
    push_audio_command(AudioCommand::IncrementRefs(source.all_sound_ids()));
}

fn post_remove(source: &mut AudioSource, entity: &Entity, _ctx: &mut GameCtxMut) {
    push_audio_command(AudioCommand::StopLoop(**entity as u64));
    push_audio_command(AudioCommand::DecrementRefs(source.all_sound_ids()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::command_queue::drain_audio_commands;
    use crate::game::Game;

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

        assert_eq!(ids, vec![
            "footstep_a".to_string(),
            "footstep_b".to_string(),
            "talk_a".to_string(),
        ]);
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
        let mut source = AudioSource::default();
        source.current = Some(SoundGroupId::Custom("Talk".to_string()));
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

        post_create(&mut source, &Entity(7), &mut ctx);

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

        post_remove(&mut source, &entity, &mut ctx);

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

}
