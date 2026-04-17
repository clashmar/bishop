mod document;
pub(crate) mod instance_sync;
pub mod prefab_editor;
mod session_state;

use engine_core::prelude::*;

pub(crate) const BLANK_PREFAB_ID: PrefabId = PrefabId(0);
pub(crate) use session_state::{
    PendingPrefabRequest, PendingPrefabTransition, PrefabSessionState, PrefabTransitionPrompt,
};

#[cfg(test)]
mod tests;
