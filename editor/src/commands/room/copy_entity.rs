// editor/src/commands/room/copy_entity.rs
use crate::EDITOR_SERVICES;
use engine_core::prelude::*;

/// Copy a snapshot of the entity and its children to the global clipboard.
/// Returns false if the entity is a Player (copying Player is not allowed).
pub fn copy_entity(ecs: &mut Ecs, entity: Entity) -> bool {
    if ecs.has::<Player>(entity) {
        return false;
    }

    let snapshot = capture_subtree(ecs, entity);
    EDITOR_SERVICES.with(|s| {
        *s.entity_clipboard.borrow_mut() = Some(snapshot);
    });
    true
}

/// Copy multiple entities and their children to the global clipboard.
/// Skips any Player entities. Returns the number of entities copied.
pub fn copy_entities(ecs: &mut Ecs, entities: &[Entity]) -> usize {
    let mut all_snapshots = Vec::new();

    for &entity in entities {
        if ecs.has::<Player>(entity) {
            continue;
        }
        let snapshot = capture_subtree(ecs, entity);
        all_snapshots.extend(snapshot);
    }

    let count = entities.len();
    if !all_snapshots.is_empty() {
        EDITOR_SERVICES.with(|s| {
            *s.entity_clipboard.borrow_mut() = Some(all_snapshots);
        });
    }
    count
}
