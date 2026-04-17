mod document;
pub(crate) mod instance_sync;
pub mod prefab_editor;

use engine_core::prelude::*;

pub(crate) const BLANK_PREFAB_ID: PrefabId = PrefabId(0);

#[cfg(test)]
mod tests;
