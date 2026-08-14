use crate::room::collider_drag::ColliderHandleDragState;
use crate::room::interactable_drag::InteractableHandleDragState;
use bishop::prelude::Vec2;
use engine_core::ecs::Entity;
use std::collections::HashSet;

/// Stores the original drag state before switching to copy mode.
pub(crate) struct PreCopyDragState {
    /// The drag anchor before entering alt-copy mode.
    pub anchor_entity: Option<Entity>,
    /// The original selection before entering alt-copy mode.
    pub selected_entities: HashSet<Entity>,
}

/// All transient mouse-interaction state for scene dragging and box selection.
#[derive(Default)]
pub(crate) struct DragState {
    /// Entity drag state.
    pub entity_drag: EntityDragState,
    /// Start position of a box selection in world coordinates.
    pub box_select_start: Option<Vec2>,
    /// Whether a box selection drag is currently active.
    pub box_select_active: bool,
    /// Collider handle drag state.
    pub collider_drag: ColliderHandleDragState,
    /// Interactable handle drag state.
    pub interactable_drag: InteractableHandleDragState,
}

/// Transient state for an active entity drag operation.
#[derive(Default)]
pub(crate) struct EntityDragState {
    /// Whether an entity drag is currently active.
    pub dragging: bool,
    /// The entity that was clicked to start the drag.
    pub anchor_entity: Option<Entity>,
    /// Offset from the anchor entity's position to the mouse at drag start.
    pub drag_offset: Vec2,
    /// Start positions of all dragged entities at the moment dragging began.
    pub drag_start_positions: Vec<(Entity, Vec2)>,
    /// The very first start positions when the drag began, used for undo commands.
    pub drag_initial_start_positions: Vec<(Entity, Vec2)>,
    /// Whether the current drag is an alt+drag copy operation.
    pub alt_copy_mode: bool,
    /// Entities created during an alt+drag copy, for the undo command.
    pub alt_copied_entities: Vec<Entity>,
    /// Original-to-copy pairs for the current alt+drag copy operation.
    pub alt_copy_pairs: Vec<(Entity, Entity)>,
    /// Original drag state before entering copy mode, used to revert on alt release.
    pub pre_copy_drag_state: Option<PreCopyDragState>,
}

/// Commands produced when an entity drag completes.
#[derive(Debug, PartialEq)]
pub(crate) enum EntityDragCommand {
    MoveOne { entity: Entity, from: Vec2, to: Vec2 },
    MoveMany { moves: Vec<(Entity, Vec2, Vec2)> },
    AltCopy { copied_entities: Vec<Entity> },
}

impl EntityDragState {
    /// Clears all transient entity drag state.
    pub(crate) fn clear(&mut self) {
        self.drag_start_positions.clear();
        self.drag_initial_start_positions.clear();
        self.anchor_entity = None;
        self.dragging = false;
        self.alt_copy_mode = false;
        self.alt_copied_entities.clear();
        self.alt_copy_pairs.clear();
        self.pre_copy_drag_state = None;
    }
}
