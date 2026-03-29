// editor/src/commands/room/paste_entity_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use crate::EDITOR_SERVICES;
use engine_core::prelude::*;
use std::collections::{HashMap, HashSet};

/// Undo-able command for pasting entities from the clipboard.
#[derive(Debug)]
pub struct PasteEntityCmd {
    room_id: RoomId,
    /// Maps old entity IDs to newly created ones.
    id_map: Option<HashMap<Entity, Entity>>,
    /// The component snapshot taken the first time the command ran.
    snapshot: Option<GroupSnapshot>,
    /// Root entities (those without parents in the snapshot) for selection.
    root_entities: Vec<Entity>,
}

impl PasteEntityCmd {
    pub fn new(room_id: RoomId) -> Self {
        Self {
            room_id,
            id_map: None,
            snapshot: None,
            root_entities: Vec::new(),
        }
    }
}

impl EditorCommand for PasteEntityCmd {
    fn execute(&mut self) {
        if self.snapshot.is_none() {
            self.snapshot = EDITOR_SERVICES.with(|s| s.entity_clipboard.borrow().clone());
        }
        let group_snapshots = match &self.snapshot {
            Some(s) => s,
            None => return,
        };

        // Find which entities are roots (have no parent in the snapshot)
        let snapshot_ids: HashSet<Entity> = group_snapshots.iter().map(|s| s.entity).collect();
        let mut root_old_ids = Vec::new();

        for snapshot in group_snapshots.iter() {
            let has_parent_in_snapshot = snapshot.components.iter().any(|comp| {
                if comp.type_name == comp_type_name::<Parent>() {
                    // Parse the parent ID from RON and check if it's in the snapshot
                    if let Ok(parent) = ron::from_str::<Parent>(&comp.ron) {
                        return snapshot_ids.contains(&parent.0);
                    }
                }
                false
            });

            if !has_parent_in_snapshot {
                root_old_ids.push(snapshot.entity);
            }
        }

        // Create new entities for each in the snapshot
        let mut map = HashMap::new();
        for snapshot in group_snapshots.iter() {
            let new_id = with_editor(|editor| {
                let ecs = &mut editor.game.ecs;
                ecs.create_entity().finish()
            });
            map.insert(snapshot.entity, new_id);
        }
        self.id_map = Some(map.clone());

        // Track the new root entity IDs
        self.root_entities = root_old_ids
            .iter()
            .filter_map(|old| map.get(old).copied())
            .collect();

        with_editor(|editor| {
            let ctx = &mut editor.game.ctx_mut();

            for snapshot in group_snapshots.iter() {
                let new_id = map[&snapshot.entity];

                for comp in snapshot.components.iter() {
                    let component_reg = inventory::iter::<ComponentRegistry>()
                        .find(|r| r.type_name == comp.type_name)
                        .expect("Component not registered");

                    let mut boxed = (component_reg.from_ron_component)(comp.ron.clone());

                    if comp.type_name == comp_type_name::<Parent>() {
                        let parent = boxed
                            .as_mut()
                            .downcast_mut::<Parent>()
                            .expect("Parent component type mismatch");

                        if let Some(&new_parent) = map.get(&parent.0) {
                            parent.0 = new_parent;
                        }
                    } else if comp.type_name == comp_type_name::<Children>() {
                        let children = boxed
                            .as_mut()
                            .downcast_mut::<Children>()
                            .expect("Children component type mismatch");

                        for child in &mut children.entities {
                            if let Some(&new_child) = map.get(child) {
                                *child = new_child;
                            }
                        }
                    }

                    (component_reg.post_create)(&mut *boxed, &new_id, ctx);
                    (component_reg.inserter)(ctx.ecs, new_id, boxed);
                }
            }

            // Select all pasted root entities
            editor.room_editor.clear_selection();
            for &root in &self.root_entities {
                editor.room_editor.add_to_selection(root);
            }
        });
    }

    fn undo(&mut self) {
        if self.id_map.is_some() {
            with_editor(|editor| {
                let ctx = &mut editor.game.ctx_mut();

                // Remove all root entities (children are removed automatically)
                for &root in &self.root_entities {
                    Ecs::remove_entity(ctx, root);
                }

                editor.room_editor.clear_selection();
            });
        }
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Room(self.room_id)
    }
}
