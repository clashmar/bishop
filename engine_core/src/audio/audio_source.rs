use crate::audio::command_queue::{AudioCommand, push_audio_command};
use crate::ecs::entity::Entity;
use crate::game::GameCtxMut;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// An ECS component that declares audio clips an entity can play.
///
/// Sounds are identified by file paths relative to `Resources/audio/` without extension.
/// Reference counts are maintained automatically via `post_create` and `post_remove` hooks,
/// ensuring sounds are loaded when the component is added and evicted when it is removed.
#[ecs_component(post_create = post_create, post_remove = post_remove)]
#[derive(Clone, Serialize, Deserialize)]
pub struct AudioSource {
    /// Sound file paths relative to `Resources/audio/` (without extension).
    pub sounds: Vec<String>,
    /// Base volume 0.0–1.0, multiplied with the SFX group volume.
    pub volume: f32,
    /// Random pitch shift range: playback speed = 1.0 ± pitch_variation.
    pub pitch_variation: f32,
    /// Random volume jitter range: playback vol = volume ± volume_variation.
    pub volume_variation: f32,
    /// Whether this source loops continuously until stopped.
    pub looping: bool,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            sounds: Vec::new(),
            volume: 1.0,
            pitch_variation: 0.0,
            volume_variation: 0.0,
            looping: false,
        }
    }
}

fn post_create(source: &mut AudioSource, _entity: &Entity, _ctx: &mut GameCtxMut) {
    push_audio_command(AudioCommand::IncrementRefs(source.sounds.clone()));
}

fn post_remove(source: &mut AudioSource, _entity: &Entity, _ctx: &mut GameCtxMut) {
    push_audio_command(AudioCommand::DecrementRefs(source.sounds.clone()));
}
