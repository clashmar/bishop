use engine_core::prefab::PrefabId;

/// Maximum number of recent prefabs persisted for the room palette.
pub const PREFAB_PALETTE_RECENT_CAP: usize = 10;

/// Persisted room prefab palette state for the active game.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PrefabPaletteState {
    /// The currently active prefab, if any.
    pub active_prefab_id: Option<PrefabId>,
    /// Recently used prefab ids, most recent first.
    pub recent_prefab_ids: Vec<PrefabId>,
}

pub(crate) fn reconcile_recent_prefab_ids(
    recent_prefab_ids: Vec<PrefabId>,
    prefab_manager: &engine_core::prefab::PrefabManager,
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
