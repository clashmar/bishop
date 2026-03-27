// editor/src/commands/room/duplicate_entities_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;
use std::collections::{HashMap, HashSet};

/// Undo-able command for duplicating selected entities.
#[derive(Debug)]
pub struct DuplicateEntitiesCmd {
    room_id: RoomId,
    /// The entities to duplicate.
    source_entities: Vec<Entity>,
    /// Maps old entity IDs to newly created ones.
    id_map: Option<HashMap<Entity, Entity>>,
    /// The component snapshot captured on first execute.
    snapshot: Option<GroupSnapshot>,
    /// Root entities created by duplication for selection/undo.
    root_entities: Vec<Entity>,
}

impl DuplicateEntitiesCmd {
    pub fn new(entities: Vec<Entity>, room_id: RoomId) -> Self {
        Self {
            room_id,
            source_entities: entities,
            id_map: None,
            snapshot: None,
            root_entities: Vec::new(),
        }
    }
}

impl EditorCommand for DuplicateEntitiesCmd {
    fn execute(&mut self) {
        // Capture snapshot on first execution
        if self.snapshot.is_none() {
            let mut all_snapshots = Vec::new();
            with_editor(|editor| {
                let ecs = &mut editor.game.ecs;
                for &entity in &self.source_entities {
                    if ecs.has::<Player>(entity) {
                        continue;
                    }
                    let snapshot = capture_subtree(ecs, entity);
                    all_snapshots.extend(snapshot);
                }
            });
            self.snapshot = Some(all_snapshots);
        }

        let group_snapshots = match &self.snapshot {
            Some(s) if !s.is_empty() => s,
            _ => return,
        };

        // Find root entities (no parent in snapshot)
        let snapshot_ids: HashSet<Entity> = group_snapshots.iter().map(|s| s.entity).collect();
        let mut root_old_ids = Vec::new();

        for snapshot in group_snapshots.iter() {
            let has_parent_in_snapshot = snapshot.components.iter().any(|comp| {
                if comp.type_name == comp_type_name::<Parent>() {
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

        // Create new entities
        let mut map = HashMap::new();
        for snapshot in group_snapshots.iter() {
            let new_id = with_editor(|editor| {
                let ecs = &mut editor.game.ecs;
                ecs.create_entity().finish()
            });
            map.insert(snapshot.entity, new_id);
        }
        self.id_map = Some(map.clone());
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

            // Select all duplicated root entities
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

                for &root in &self.root_entities {
                    Ecs::remove_entity(ctx, root);
                }

                // Restore original selection
                editor.room_editor.clear_selection();
                for &entity in &self.source_entities {
                    editor.room_editor.add_to_selection(entity);
                }
            });
        }
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Room(self.room_id)
    }
}
