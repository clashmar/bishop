mod document;
pub(crate) mod instance_sync;
mod palette;
pub mod prefab_editor;
mod session_state;

use engine_core::prelude::*;

pub(crate) const BLANK_PREFAB_ID: PrefabId = PrefabId(0);
pub(crate) use palette::reconcile_recent_prefab_ids;
pub(crate) use session_state::{
    PendingPrefabTransition, PrefabSessionState, PrefabTransitionPrompt,
};

#[cfg(test)]
pub(crate) mod tests;
