mod core;
mod policy;

use crate::app::EditorMode;
use engine_core::prelude::*;

pub use core::SceneInspector;
pub use policy::{
    is_scene_component_hidden_in_prefab, linked_prefab_instance_state_for_scene_inspector,
};

/// Supported linked-prefab actions emitted from the room inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenePrefabAction {
    OpenPrefabEditor,
    UnlinkInstance,
    ApplyInstanceToPrefab,
    RevertInstanceToPrefab,
}

/// Concrete linked-prefab action request emitted from the inspector UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScenePrefabActionRequest {
    pub action: ScenePrefabAction,
    pub selected_entity: Entity,
    pub root_entity: Entity,
    pub prefab_id: PrefabId,
}

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
    /// Linked-prefab action triggered from the inspector, if any.
    pub prefab_action: Option<ScenePrefabActionRequest>,
    /// Whether the prefab-mode empty state requested the prefab picker.
    pub open_prefab_picker: bool,
}

/// Full draw result emitted by the shared inspector core.
#[derive(Clone, Debug, Default)]
pub struct SceneInspectorDrawResult {
    /// Behavioral output emitted by the inspector draw.
    pub output: SceneInspectorOutput,
    /// Interactive rectangles used by the thin host wrapper for hit-testing.
    pub interactive_rects: Vec<Rect>,
}

impl SceneInspectorDrawResult {
    /// Creates a full draw result from inspector output and interactive rectangles.
    pub fn new(output: SceneInspectorOutput, interactive_rects: Vec<Rect>) -> Self {
        Self {
            output,
            interactive_rects,
        }
    }
}

/// Scene entity creation request emitted by the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneCreateRequest {
    /// Parent for the new entity, if one should be assigned immediately.
    pub parent: Option<Entity>,
}
