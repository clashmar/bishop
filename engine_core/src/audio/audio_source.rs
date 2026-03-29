use crate::audio::command_queue::{AudioCommand, push_audio_command};
use crate::ecs::entity::Entity;
use crate::game::GameCtxMut;
use ecs_component::ecs_component;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
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
pub(crate) fn test_post_create(source: &mut AudioSource, entity: &Entity, ctx: &mut GameCtxMut) {
    post_create(source, entity, ctx);
}

#[cfg(test)]
pub(crate) fn test_post_remove(source: &mut AudioSource, entity: &Entity, ctx: &mut GameCtxMut) {
    post_remove(source, entity, ctx);
}
