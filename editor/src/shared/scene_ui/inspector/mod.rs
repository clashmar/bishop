pub mod entity_inspector;
mod policy;

use crate::app::EditorMode;
use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use engine_core::ecs::*;

pub use entity_inspector::EntityInspector;
pub use policy::{
    is_scene_component_hidden_in_prefab, linked_prefab_instance_state_for_scene_inspector,
};

/// Content that the inspector shell renders.
/// The shell handles scrolling, clipping, and background.
pub trait InspectorContent {
    /// Height of the fixed header area (buttons, dropdowns). 0 if none.
    fn header_height(&self) -> f32 {
        0.0
    }

    /// Draw fixed header controls above the scrollable area.
    fn draw_header(
        &mut self,
        _ctx: &mut WgpuContext,
        _rect: Rect,
        _blocked: bool,
        _game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        InspectorOutput::default()
    }

    /// Draw the scrollable module list.
    fn draw_modules(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        blocked: bool,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput;

    /// Total scrollable content height.
    fn total_content_height(&self, game_ctx: &mut GameCtxMut, insp_ctx: &InspectorContext) -> f32;

    /// Interactive rects for hit-testing.
    fn interactive_rects(&self) -> Vec<Rect> {
        vec![]
    }

    /// Clears interactive rects for the current frame.
    fn clear_interactive_rects(&mut self) {}
}

/// Supported linked-prefab actions emitted from the room inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefabAction {
    OpenPrefabEditor,
    UnlinkInstance,
    ApplyInstanceToPrefab,
    RevertInstanceToPrefab,
}

/// Concrete linked-prefab action request emitted from the inspector UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrefabActionRequest {
    pub action: PrefabAction,
    pub selected_entity: Entity,
    pub root_entity: Entity,
    pub prefab_id: PrefabId,
}

/// Per-frame scene-inspector behavior flags and host state.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectorContext {
    /// Command scope used for inspector-triggered undoable actions.
    pub command_mode: EditorMode,
    /// Whether linked prefab metadata is visible in the inspector.
    pub show_linked_prefab_metadata: bool,
    /// Whether room/runtime-only components should be hidden from addable lists.
    pub hide_room_only_components: bool,
    /// Parent to use for the selected-entity `+ Entity` affordance.
    pub selected_create_parent: Option<Entity>,
    /// Name for the game being edited, when in Game mode.
    pub game_name: Option<String>,
    /// Event tags currently known to the editor.
    pub event_tags: Vec<String>,
}

/// Per-frame output emitted by the shared inspector UI.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InspectorOutput {
    /// Create request triggered by `+ Entity`, if any.
    pub create_request: Option<CreateRequest>,
    /// Camera creation request triggered by `+ Camera`, carrying grid size.
    pub create_camera_request: Option<f32>,
    /// Linked-prefab action triggered from the inspector, if any.
    pub prefab_action: Option<PrefabActionRequest>,
    /// Host-level action emitted by Game/World inspector content.
    pub host_action: Option<InspectorHostAction>,
    /// Whether the prefab-mode empty state requested the prefab picker.
    pub open_prefab_picker: bool,
    /// Whether the prefab-mode empty state requested prefab deletion.
    pub delete_prefab: bool,
    /// Whether the host should regenerate global event-tag output.
    pub refresh_event_tags: bool,
}

/// Actions that require host-level coordination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InspectorHostAction {
    RenameGame(String),
    RenameWorld(String),
    /// Requests the world editor to centre its camera on the given entity.
    FocusWorldEditor(Entity),
}

impl InspectorOutput {
    pub fn merge(&mut self, other: Self) {
        if self.create_request.is_none() {
            self.create_request = other.create_request;
        }
        if self.create_camera_request.is_none() {
            self.create_camera_request = other.create_camera_request;
        }
        if self.prefab_action.is_none() {
            self.prefab_action = other.prefab_action;
        }
        if self.host_action.is_none() {
            self.host_action = other.host_action;
        }
        self.open_prefab_picker |= other.open_prefab_picker;
        self.delete_prefab |= other.delete_prefab;
        self.refresh_event_tags |= other.refresh_event_tags;
    }
}


/// Scene entity creation request emitted by the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateRequest {
    /// Parent for the new entity, if one should be assigned immediately.
    pub parent: Option<Entity>,
}
