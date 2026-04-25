use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::prefab::instance_sync::{
    capture_linked_prefab_instance_snapshots, instance_prefab_differs,
    linked_prefab_instance_roots, linked_prefab_reference, replace_linked_instances_with_snapshots,
    sync_prefab_overrides_for_root,
};
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command that commits a linked room instance back into its prefab asset.
#[derive(Debug)]
pub struct ApplyInstanceToPrefabCmd {
    entity: Entity,
    mode: EditorMode,
    preferred_selection: Entity,
    root_entity: Option<Entity>,
    prefab_id: Option<PrefabId>,
    previous_prefab: Option<PrefabAsset>,
    previous_snapshots: Vec<GroupSnapshot>,
}

impl ApplyInstanceToPrefabCmd {
    pub fn new(entity: Entity, mode: EditorMode) -> Self {
        Self {
            entity,
            mode,
            preferred_selection: entity,
            root_entity: None,
            prefab_id: None,
            previous_prefab: None,
            previous_snapshots: Vec::new(),
        }
    }
}

impl EditorCommand for ApplyInstanceToPrefabCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let Some(reference) = linked_prefab_reference(&editor.game.ecs, self.entity) else {
                return;
            };
            let Some(prefab) = editor
                .game
                .prefab_manager
                .prefabs
                .get(&reference.prefab_id)
                .cloned()
            else {
                return;
            };

            self.root_entity = Some(reference.root_entity);
            self.prefab_id = Some(reference.prefab_id);
            self.previous_prefab = Some(prefab.clone());
            self.previous_snapshots =
                capture_linked_prefab_instance_snapshots(&mut editor.game.ecs, reference.prefab_id);

            if !instance_prefab_differs(&mut editor.game.ecs, &prefab, reference.root_entity) {
                restore_room_selection(editor, self.preferred_selection, reference.root_entity);
                return;
            }

            let updated_prefab = capture_prefab_with_existing(
                &mut editor.game.ecs,
                reference.root_entity,
                prefab.id,
                prefab.name.clone(),
                Some(&prefab),
            );

            if let Err(error) = editor.game.prefab_manager.save_prefab_and_sync(
                &editor.game.name,
                &mut editor.game.asset_registry,
                &updated_prefab,
                None,
            ) {
                onscreen_error!("Could not save prefab: {error}");
                return;
            }

            let roots = linked_prefab_instance_roots(&editor.game.ecs, updated_prefab.id);
            for root_entity in roots {
                let room_id = editor
                    .game
                    .ecs
                    .get::<CurrentRoom>(root_entity)
                    .map(|room| room.0);
                let mut ctx = editor.game.ctx_mut();
                refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, room_id);
            }

            let roots = linked_prefab_instance_roots(&editor.game.ecs, updated_prefab.id);
            for root_entity in roots {
                sync_prefab_overrides_for_root(&mut editor.game.ecs, &updated_prefab, root_entity);
            }

            restore_room_selection(editor, self.preferred_selection, reference.root_entity);
        });
    }

    fn undo(&mut self) {
        let Some(prefab_id) = self.prefab_id else {
            return;
        };
        let Some(root_entity) = self.root_entity else {
            return;
        };
        let Some(previous_prefab) = self.previous_prefab.as_ref() else {
            return;
        };

        with_editor(|editor| {
            if let Err(error) = editor.game.prefab_manager.save_prefab_and_sync(
                &editor.game.name,
                &mut editor.game.asset_registry,
                previous_prefab,
                None,
            ) {
                onscreen_error!("Could not restore prefab: {error}");
                return;
            }

            replace_linked_instances_with_snapshots(
                &mut editor.game,
                prefab_id,
                &self.previous_snapshots,
            );
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
