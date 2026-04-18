use crate::app::Editor;
use crate::prefab::prefab_editor::{PrefabRoomSyncState, StagedPrefabState};
use engine_core::prelude::*;
use std::collections::HashSet;

impl Editor {
    pub(crate) fn reconcile_active_prefab_room_preview(&mut self) {
        let Some(staged_state) = self.active_prefab_staged_state() else {
            return;
        };

        let needs_sync = self.prefab_editor.as_ref().is_some_and(|prefab_editor| {
            prefab_editor.last_room_synced_state.staged_prefab != staged_state
        });
        if !needs_sync {
            return;
        }

        self.reconcile_prefab_room_state(staged_state);
    }

    pub(crate) fn reconcile_prefab_room_state(&mut self, target_state: StagedPrefabState) {
        let Some(prefab_editor) = self.prefab_editor.as_mut() else {
            return;
        };

        let preserved_snapshots = prefab_editor
            .last_room_synced_state
            .linked_instance_snapshots
            .clone();
        let prefab_id = prefab_editor.prefab_id;

        match &target_state {
            StagedPrefabState::PrefabAsset(prefab) => {
                restore_prefab_instance_snapshots(&mut self.game, prefab_id, &preserved_snapshots);
                refresh_linked_prefab_instances(&mut self.game, prefab);
                prefab_editor.last_room_synced_state =
                    capture_prefab_room_sync_state(&mut self.game.ecs, prefab_id, prefab.clone());
            }
            StagedPrefabState::Empty => {
                let snapshots = remove_prefab_and_linked_instances(
                    &mut self.game,
                    &mut self.room_editor,
                    prefab_id,
                );
                prefab_editor.last_room_synced_state = PrefabRoomSyncState {
                    staged_prefab: StagedPrefabState::Empty,
                    linked_instance_snapshots: if snapshots.is_empty() {
                        preserved_snapshots
                    } else {
                        snapshots
                    },
                };
            }
        }
    }
}

pub(super) fn relink_room_subtree_to_prefab(
    game: &mut Game,
    root_entity: Entity,
    prefab: &PrefabAsset,
) -> Option<Entity> {
    let root_position = game
        .ecs
        .get::<Transform>(root_entity)
        .map(|transform| transform.position)
        .unwrap_or_default();
    let parent_entity = get_parent(&game.ecs, root_entity);
    let room_id = game.ecs.get::<CurrentRoom>(root_entity).map(|room| room.0);

    let replacement_root = {
        let mut ctx = game.ctx_mut();
        instantiate_prefab(&mut ctx, prefab, root_position, room_id)
    };

    if replacement_root == Entity::null() {
        return None;
    }

    if let Some(parent_entity) = parent_entity {
        set_parent(&mut game.ecs, replacement_root, parent_entity);
    }

    let mut ctx = game.ctx_mut();
    Ecs::remove_entity(&mut ctx, root_entity);
    Some(replacement_root)
}

fn refresh_linked_prefab_instances(game: &mut Game, prefab: &PrefabAsset) {
    let roots = linked_prefab_instance_roots(&game.ecs, prefab.id);

    for root_entity in roots {
        let room_id = game.ecs.get::<CurrentRoom>(root_entity).map(|room| room.0);
        let mut ctx = game.ctx_mut();
        refresh_prefab_instance(&mut ctx, root_entity, prefab, room_id);
    }
}

fn linked_prefab_instance_roots(ecs: &Ecs, prefab_id: PrefabId) -> Vec<Entity> {
    ecs.get_store::<PrefabInstanceRoot>()
        .data
        .iter()
        .filter_map(|(&entity, root)| (root.prefab_id == prefab_id).then_some(entity))
        .collect()
}

pub(super) fn capture_prefab_room_sync_state(
    ecs: &mut Ecs,
    prefab_id: PrefabId,
    prefab: PrefabAsset,
) -> PrefabRoomSyncState {
    PrefabRoomSyncState {
        staged_prefab: StagedPrefabState::PrefabAsset(prefab),
        linked_instance_snapshots: capture_linked_prefab_instance_snapshots(ecs, prefab_id),
    }
}

pub(super) fn sync_prefab_stage_instance_metadata(
    ecs: &mut Ecs,
    root_entity: Entity,
    prefab: &PrefabAsset,
) {
    let subtree = capture_subtree(ecs, root_entity);
    if subtree.len() != prefab.nodes.len() {
        return;
    }

    for (snapshot, node) in subtree.into_iter().zip(prefab.nodes.iter()) {
        ecs.add_component_to_entity(
            snapshot.entity,
            PrefabInstanceNode {
                prefab_id: prefab.id,
                node_id: node.node_id,
                root_entity,
            },
        );
    }

    ecs.add_component_to_entity(
        root_entity,
        PrefabInstanceRoot {
            prefab_id: prefab.id,
        },
    );
}

fn capture_linked_prefab_instance_snapshots(
    ecs: &mut Ecs,
    prefab_id: PrefabId,
) -> Vec<GroupSnapshot> {
    linked_prefab_instance_roots(ecs, prefab_id)
        .into_iter()
        .map(|root| capture_subtree(ecs, root))
        .collect()
}

fn restore_prefab_instance_snapshots(
    game: &mut Game,
    prefab_id: PrefabId,
    snapshots: &[GroupSnapshot],
) {
    let existing_roots = linked_prefab_instance_roots(&game.ecs, prefab_id)
        .into_iter()
        .collect::<HashSet<_>>();

    for snapshot in snapshots {
        let Some(root_entity) = snapshot.first().map(|entity| entity.entity) else {
            continue;
        };
        if existing_roots.contains(&root_entity) {
            continue;
        }

        let mut ctx = game.ctx_mut();
        restore_subtree(&mut ctx, snapshot);
    }
}

fn remove_prefab_and_linked_instances(
    game: &mut Game,
    room_editor: &mut crate::room::room_editor::RoomEditor,
    prefab_id: PrefabId,
) -> Vec<GroupSnapshot> {
    let roots = linked_prefab_instance_roots(&game.ecs, prefab_id);
    let mut removed_entities = HashSet::new();
    let mut snapshots = Vec::with_capacity(roots.len());

    for root_entity in roots {
        let snapshot = capture_subtree(&mut game.ecs, root_entity);
        removed_entities.extend(snapshot.iter().map(|entity| entity.entity));
        snapshots.push(snapshot);

        let mut ctx = game.ctx_mut();
        Ecs::remove_entity(&mut ctx, root_entity);
    }

    room_editor
        .selected_entities
        .retain(|entity| !removed_entities.contains(entity));
    if !room_editor
        .inspector
        .target
        .is_some_and(|entity| removed_entities.contains(&entity))
    {
        return snapshots;
    }

    room_editor.inspector.set_target(None);
    snapshots
}
