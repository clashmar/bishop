// editor/src/commands/room/update_component_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;
use std::any::Any;

/// Non-persisted component state that should survive inspector undo snapshots.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ComponentTransientState {
    #[default]
    None,
    AudioSource {
        current: Option<SoundGroupId>,
    },
}

/// Captures transient editor state for a component clone before serializing it.
pub fn capture_component_transient_state(
    type_name: &'static str,
    component: &dyn Any,
) -> ComponentTransientState {
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
                let mut old = (reg.clone)(ctx.ecs, entity);
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

        let mut restored = AudioSource::default();
        restored.groups = source.groups.clone();
        restored.current = None;

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
}
