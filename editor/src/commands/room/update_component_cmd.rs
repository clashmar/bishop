// editor/src/commands/room/update_component_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;
use std::any::Any;
use std::collections::HashMap;

/// Non-persisted component state that should survive inspector undo snapshots.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ComponentTransientState {
    #[default]
    None,
    Animation {
        current: Option<ClipId>,
        sprite_cache: HashMap<ClipId, SpriteId>,
    },
    AudioSource {
        current: Option<SoundGroupId>,
    },
}

/// Captures transient editor state for a component clone before serializing it.
pub fn capture_component_transient_state(
    type_name: &'static str,
    component: &dyn Any,
) -> ComponentTransientState {
    if type_name == Animation::TYPE_NAME {
        return component
            .downcast_ref::<Animation>()
            .map(|animation| ComponentTransientState::Animation {
                current: animation.current.clone(),
                sprite_cache: animation.sprite_cache.clone(),
            })
            .unwrap_or_default();
    }

    if type_name == AudioSource::TYPE_NAME {
        return component
            .downcast_ref::<AudioSource>()
            .map(|source| ComponentTransientState::AudioSource {
                current: source.current.clone(),
            })
            .unwrap_or_default();
    }

    ComponentTransientState::None
}

/// Restores transient editor state onto a deserialized component snapshot.
pub fn restore_component_transient_state(
    type_name: &'static str,
    component: &mut dyn Any,
    transient_state: &ComponentTransientState,
) {
    if type_name == Animation::TYPE_NAME {
        let Some(animation) = component.downcast_mut::<Animation>() else {
            return;
        };

        let ComponentTransientState::Animation {
            current,
            sprite_cache,
        } = transient_state
        else {
            return;
        };

        animation.current = current
            .clone()
            .filter(|clip_id| animation.clips.contains_key(clip_id));
        animation.sprite_cache = sprite_cache
            .iter()
            .filter(|(clip_id, sprite_id)| {
                sprite_id.0 != 0 && animation.clips.contains_key(*clip_id)
            })
            .map(|(clip_id, sprite_id)| (clip_id.clone(), *sprite_id))
            .collect();
        return;
    }

    if type_name != AudioSource::TYPE_NAME {
        return;
    }

    let Some(source) = component.downcast_mut::<AudioSource>() else {
        return;
    };

    let ComponentTransientState::AudioSource { current } = transient_state else {
        return;
    };

    source.current = current
        .clone()
        .filter(|group_id| source.groups.contains_key(group_id));
}

fn should_reapply_component(
    current_ron: &str,
    incoming_ron: &str,
    current_transient_state: &ComponentTransientState,
    incoming_transient_state: &ComponentTransientState,
) -> bool {
    current_ron != incoming_ron || current_transient_state != incoming_transient_state
}

/// Undo-able command for editing a single component field via the inspector.
#[derive(Debug)]
pub struct UpdateComponentCmd {
    entity: Entity,
    room_id: RoomId,
    type_name: &'static str,
    old_ron: String,
    new_ron: String,
    old_transient_state: ComponentTransientState,
    new_transient_state: ComponentTransientState,
}

impl UpdateComponentCmd {
    pub fn new(
        entity: Entity,
        room_id: RoomId,
        type_name: &'static str,
        old_ron: String,
        new_ron: String,
        old_transient_state: ComponentTransientState,
        new_transient_state: ComponentTransientState,
    ) -> Self {
        Self {
            entity,
            room_id,
            type_name,
            old_ron,
            new_ron,
            old_transient_state,
            new_transient_state,
        }
    }

    fn apply(
        entity: Entity,
        type_name: &'static str,
        ron: String,
        transient_state: &ComponentTransientState,
        editor: &mut crate::Editor,
    ) {
        let ctx = &mut editor.game.ctx_mut();
        if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
            if (reg.has)(ctx.ecs, entity) {
                let old = (reg.clone)(ctx.ecs, entity);
                let current_ron = (reg.to_ron_component)(old.as_ref());
                let current_transient_state =
                    capture_component_transient_state(type_name, old.as_ref());

                if !should_reapply_component(
                    &current_ron,
                    &ron,
                    &current_transient_state,
                    transient_state,
                ) {
                    return;
                }

                let mut old = old;
                (reg.post_remove)(&mut *old, &entity, ctx);
            }
            let mut boxed = (reg.from_ron_component)(ron);
            restore_component_transient_state(type_name, boxed.as_mut(), transient_state);
            (reg.post_create)(&mut *boxed, &entity, ctx);
            (reg.inserter)(ctx.ecs, entity, boxed);
        }
    }
}

impl EditorCommand for UpdateComponentCmd {
    fn execute(&mut self) {
        let type_name = self.type_name;
        let ron = self.new_ron.clone();
        let entity = self.entity;
        let transient_state = self.new_transient_state.clone();
        with_editor(|editor| Self::apply(entity, type_name, ron, &transient_state, editor));
    }

    fn undo(&mut self) {
        let type_name = self.type_name;
        let ron = self.old_ron.clone();
        let entity = self.entity;
        let transient_state = self.old_transient_state.clone();
        with_editor(|editor| Self::apply(entity, type_name, ron, &transient_state, editor));
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Room(self.room_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_reapply_when_component_snapshot_is_identical() {
        let transient_state = ComponentTransientState::None;

        assert!(!should_reapply_component(
            "Sprite(sprite:1)",
            "Sprite(sprite:1)",
            &transient_state,
            &transient_state,
        ));
    }

    #[test]
    fn reapplies_when_component_snapshot_differs() {
        let transient_state = ComponentTransientState::None;

        assert!(should_reapply_component(
            "Sprite(sprite:1)",
            "Sprite(sprite:2)",
            &transient_state,
            &transient_state,
        ));
    }

    #[test]
    fn audio_source_transient_state_restores_selected_group() {
        let selected_group = SoundGroupId::Custom("Jump".to_string());
        let mut source = AudioSource::default();
        source
            .groups
            .insert(selected_group.clone(), AudioGroup::default());
        source.groups.insert(
            SoundGroupId::Custom("DoubleJump".to_string()),
            AudioGroup::default(),
        );
        source.current = Some(selected_group.clone());

        let snapshot = capture_component_transient_state(AudioSource::TYPE_NAME, &source);

        let mut restored = AudioSource {
            groups: source.groups.clone(),
            ..Default::default()
        };

        restore_component_transient_state(AudioSource::TYPE_NAME, &mut restored, &snapshot);

        assert_eq!(restored.current, Some(selected_group));
    }

    #[test]
    fn audio_source_transient_state_ignores_missing_selected_group() {
        let mut source = AudioSource::default();
        source.groups.insert(
            SoundGroupId::Custom("Jump".to_string()),
            AudioGroup::default(),
        );
        source.current = Some(SoundGroupId::Custom("Jump".to_string()));

        let snapshot = capture_component_transient_state(AudioSource::TYPE_NAME, &source);

        let mut restored = AudioSource::default();
        restored.groups.insert(
            SoundGroupId::Custom("DoubleJump".to_string()),
            AudioGroup::default(),
        );

        restore_component_transient_state(AudioSource::TYPE_NAME, &mut restored, &snapshot);

        assert_eq!(restored.current, None);
    }

    #[test]
    fn animation_transient_state_restores_selected_clip_and_cache() {
        let mut animation = Animation::default();
        animation.clips.insert(ClipId::Idle, ClipDef::default());
        animation.clips.insert(ClipId::Run, ClipDef::default());
        animation.current = Some(ClipId::Run);
        animation.sprite_cache.insert(ClipId::Idle, SpriteId(3));
        animation.sprite_cache.insert(ClipId::Run, SpriteId(4));

        let snapshot = capture_component_transient_state(Animation::TYPE_NAME, &animation);

        let mut restored = Animation {
            clips: animation.clips.clone(),
            ..Default::default()
        };

        restore_component_transient_state(Animation::TYPE_NAME, &mut restored, &snapshot);

        assert_eq!(restored.current, Some(ClipId::Run));
        assert_eq!(restored.sprite_cache.get(&ClipId::Idle), Some(&SpriteId(3)));
        assert_eq!(restored.sprite_cache.get(&ClipId::Run), Some(&SpriteId(4)));
    }

    #[test]
    fn animation_transient_state_filters_missing_clips_from_restore() {
        let mut animation = Animation::default();
        animation.clips.insert(ClipId::Idle, ClipDef::default());
        animation.clips.insert(ClipId::Run, ClipDef::default());
        animation.current = Some(ClipId::Run);
        animation.sprite_cache.insert(ClipId::Idle, SpriteId(5));
        animation.sprite_cache.insert(ClipId::Run, SpriteId(6));

        let snapshot = capture_component_transient_state(Animation::TYPE_NAME, &animation);

        let mut restored = Animation::default();
        restored.clips.insert(ClipId::Idle, ClipDef::default());

        restore_component_transient_state(Animation::TYPE_NAME, &mut restored, &snapshot);

        assert_eq!(restored.current, None);
        assert_eq!(restored.sprite_cache.len(), 1);
        assert_eq!(restored.sprite_cache.get(&ClipId::Idle), Some(&SpriteId(5)));
        assert!(!restored.sprite_cache.contains_key(&ClipId::Run));
    }
}
