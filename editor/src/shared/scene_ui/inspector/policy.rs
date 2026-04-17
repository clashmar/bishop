use crate::prefab::instance_sync::{linked_prefab_instance_state, LinkedPrefabInstanceState};
use engine_core::prelude::*;

/// Returns room-inspector state for linked prefab instances when that metadata is enabled.
pub fn linked_prefab_instance_state_for_scene_inspector(
    show_linked_prefab_metadata: bool,
    ecs: &mut Ecs,
    prefab_library: &PrefabLibrary,
    entity: Entity,
) -> Option<LinkedPrefabInstanceState> {
    show_linked_prefab_metadata
        .then(|| linked_prefab_instance_state(ecs, prefab_library, entity))?
}

/// Returns whether a component type should be hidden from prefab scene editing.
pub fn is_scene_component_hidden_in_prefab(type_name: &str) -> bool {
    type_name == comp_type_name::<CurrentRoom>()
        || type_name == comp_type_name::<RoomCamera>()
        || type_name == comp_type_name::<PlayerProxy>()
        || type_name == comp_type_name::<Player>()
        || type_name == comp_type_name::<Global>()
}
