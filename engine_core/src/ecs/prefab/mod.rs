mod capture;
mod component_sync;
mod instance;

pub use crate::ecs::components::prefab_instance::{
    PrefabInstanceNode, PrefabInstanceRoot, PrefabOverrides,
};
pub use capture::{capture_prefab, capture_prefab_with_existing};
pub use instance::{instantiate_prefab, refresh_prefab_instance};

#[cfg(test)]
mod tests;
