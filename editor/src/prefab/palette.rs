use crate::storage::editor_storage::PREFAB_PALETTE_RECENT_CAP;
use engine_core::prelude::*;

pub(crate) fn reconcile_recent_prefab_ids(
    recent_prefab_ids: Vec<PrefabId>,
    prefab_manager: &PrefabManager,
) -> Vec<PrefabId> {
    recent_prefab_ids
        .into_iter()
        .filter(|prefab_id| prefab_manager.prefabs.contains_key(prefab_id))
        .fold(Vec::new(), |mut ids, prefab_id| {
            if !ids.contains(&prefab_id) && ids.len() < PREFAB_PALETTE_RECENT_CAP {
                ids.push(prefab_id);
            }
            ids
        })
}
