use crate::shared::scene_ui::prefab_link::{linked_prefab_display, PrefabLinkDisplay};
use engine_core::prelude::*;

/// Returns linked-prefab metadata for scene inspector UI when that metadata is enabled.
pub(crate) fn linked_prefab_metadata_for_scene_inspector(
    show_linked_prefab_metadata: bool,
    ecs: &Ecs,
    prefab_library: &PrefabLibrary,
    entity: Entity,
) -> Option<PrefabLinkDisplay> {
    show_linked_prefab_metadata.then(|| linked_prefab_display(ecs, prefab_library, entity))?
}

/// Returns whether a component type should be hidden from prefab scene editing.
pub(crate) fn is_scene_component_hidden_in_prefab(type_name: &str) -> bool {
    type_name == comp_type_name::<CurrentRoom>()
        || type_name == comp_type_name::<RoomCamera>()
        || type_name == comp_type_name::<PlayerProxy>()
        || type_name == comp_type_name::<Player>()
        || type_name == comp_type_name::<Global>()
}
