use crate::app::EditorMode;
use engine_core::prelude::*;

/// Per-frame scene-inspector behavior flags and host state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneInspectorContext {
    /// Command scope used for inspector-triggered undoable actions.
    pub command_mode: EditorMode,
    /// Whether linked prefab metadata is visible in the inspector.
    pub show_linked_prefab_metadata: bool,
    /// Whether room/runtime-only components should be hidden from addable lists.
    pub hide_room_only_components: bool,
    /// Parent to use for the selected-entity `+ Entity` affordance.
    pub selected_create_parent: Option<Entity>,
    /// Empty-state behavior when no entity is selected.
    pub empty_state: SceneEmptyInspectorBehavior,
}

/// Empty-state behavior variants for room and prefab hosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneEmptyInspectorBehavior {
    /// Room-mode empty state with root-entity, camera, and darkness controls.
    Room,
    /// Prefab-mode empty state with a fallback parent/root for new entities.
    Prefab {
        /// Parent to use when the prefab empty-state `+ Entity` action is clicked.
        fallback_parent: Option<Entity>,
    },
}

/// Per-frame output emitted by the shared inspector UI.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneInspectorOutput {
    /// Create request triggered by `+ Entity`, if any.
    pub create_request: Option<SceneCreateRequest>,
}

/// Scene entity creation request emitted by the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneCreateRequest {
    /// Parent for the new entity, if one should be assigned immediately.
    pub parent: Option<Entity>,
}
