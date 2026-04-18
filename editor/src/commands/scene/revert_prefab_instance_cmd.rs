use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::prefab::instance_sync::{linked_prefab_reference, sync_prefab_overrides_for_root};
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command that restores a linked prefab instance from the saved prefab asset.
#[derive(Debug)]
pub struct RevertPrefabInstanceCmd {
    entity: Entity,
    mode: EditorMode,
    saved_snapshot: Option<GroupSnapshot>,
    root_entity: Option<Entity>,
    preferred_selection: Entity,
}

impl RevertPrefabInstanceCmd {
    pub fn new(entity: Entity, mode: EditorMode) -> Self {
        Self {
            entity,
            mode,
            saved_snapshot: None,
            root_entity: None,
            preferred_selection: entity,
        }
    }
}

impl EditorCommand for RevertPrefabInstanceCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let Some(reference) = linked_prefab_reference(&editor.game.ecs, self.entity) else {
                return;
            };
            let Some(prefab) = editor
                .game
                .prefab_library
                .prefabs
                .get(&reference.prefab_id)
                .cloned()
            else {
                return;
            };

            self.root_entity = Some(reference.root_entity);
            self.saved_snapshot =
                Some(capture_subtree(&mut editor.game.ecs, reference.root_entity));

            for entity in self
                .saved_snapshot
                .as_ref()
                .into_iter()
                .flatten()
                .map(|snapshot| snapshot.entity)
            {
                editor
                    .game
                    .ecs
                    .get_store_mut::<PrefabOverrides>()
                    .remove(entity);
            }

            {
                let room_id = editor
                    .game
                    .ecs
                    .get::<CurrentRoom>(reference.root_entity)
                    .map(|room| room.0);
                let mut ctx = editor.game.ctx_mut();
                refresh_prefab_instance(&mut ctx, reference.root_entity, &prefab, room_id);
            }

            sync_prefab_overrides_for_root(&mut editor.game.ecs, &prefab, reference.root_entity);
            restore_room_selection(editor, self.preferred_selection, reference.root_entity);
        });
    }

    fn undo(&mut self) {
        let Some(root_entity) = self.root_entity else {
            return;
        };
        let Some(saved_snapshot) = self.saved_snapshot.as_ref() else {
            return;
        };

        with_editor(|editor| {
            if editor.game.ecs.has::<Transform>(root_entity) {
                let mut ctx = editor.game.ctx_mut();
                Ecs::remove_entity(&mut ctx, root_entity);
            }

            {
                let mut ctx = editor.game.ctx_mut();
                restore_subtree(&mut ctx, saved_snapshot);
            }

            restore_room_selection(editor, self.preferred_selection, root_entity);
        });
    }

    fn mode(&self) -> EditorMode {
        self.mode
    }
}

fn restore_room_selection(editor: &mut crate::Editor, preferred: Entity, root_entity: Entity) {
    let selected = if editor.game.ecs.has::<Transform>(preferred) {
        Some(preferred)
    } else if editor.game.ecs.has::<Transform>(root_entity) {
        Some(root_entity)
    } else {
        None
    };

    editor.room_editor.set_selected_entity(selected);
}
