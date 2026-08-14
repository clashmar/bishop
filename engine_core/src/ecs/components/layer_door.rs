use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::{CurrentRoom, Interactable, Transform};
use crate::worlds::{RoomLayer, World};
use bishop::prelude::Rect;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Same-position doorway that swaps between front and back room layers.
#[ecs_component(deps = [Interactable])]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Reflect, PartialEq)]
#[serde(default)]
pub struct LayerDoor {
    /// Whether this doorway can currently be used.
    pub usable: bool,
    /// How visible the doorway remains while the player is on the back layer.
    pub alpha: f32,
}

impl Default for LayerDoor {
    fn default() -> Self {
        Self {
            usable: true,
            alpha: 0.4,
        }
    }
}

/// Why one authored `LayerDoor` should show an editor warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerDoorValidationIssue {
    MustBeOnFrontLayer,
    RoomHasNoBackLayer,
    MissingInteractable,
    InteractableOutsideBackBounds,
}

impl LayerDoorValidationIssue {
    /// Returns a human-readable warning message for the editor UI.
    pub fn message(self) -> &'static str {
        match self {
            LayerDoorValidationIssue::MustBeOnFrontLayer => {
                "Layer door must be on the front layer"
            }
            LayerDoorValidationIssue::RoomHasNoBackLayer => {
                "Layer door requires the room to have a back layer"
            }
            LayerDoorValidationIssue::MissingInteractable => {
                "Layer door requires an Interactable component"
            }
            LayerDoorValidationIssue::InteractableOutsideBackBounds => {
                "Layer door Interactable area must be inside back-layer bounds"
            }
        }
    }
}

/// Validates one authored `LayerDoor` against room-layer authoring rules.
pub fn validate_layer_door(
    ecs: &Ecs,
    world: &World,
    entity: Entity,
) -> Result<(), LayerDoorValidationIssue> {
    let Some(current_room) = ecs.get::<CurrentRoom>(entity).copied() else {
        debug_assert!(false, "LayerDoor missing CurrentRoom for {entity:?}");
        return Ok(());
    };
    if current_room.layer != RoomLayer::Front {
        return Err(LayerDoorValidationIssue::MustBeOnFrontLayer);
    }

    let Some(room) = world.get_room(current_room.room_id) else {
        debug_assert!(false, "LayerDoor room {:?} missing for {entity:?}", current_room.room_id);
        return Ok(());
    };
    if room.current_variant().layers.back.is_none() {
        return Err(LayerDoorValidationIssue::RoomHasNoBackLayer);
    }

    let Some(interactable) = ecs.get::<Interactable>(entity) else {
        return Err(LayerDoorValidationIssue::MissingInteractable);
    };
    let Some(transform) = ecs.get::<Transform>(entity) else {
        debug_assert!(false, "LayerDoor missing Transform for {entity:?}");
        return Ok(());
    };

    let back_bounds = room
        .current_variant()
        .layers
        .effective_back_bounds(room.world_rect(world.grid_size));
    if !interactable.area_contained_in_bounds(transform.position, &back_bounds) {
        return Err(LayerDoorValidationIssue::InteractableOutsideBackBounds);
    }

    Ok(())
}

/// Returns true when a layer door's usable area overlaps any active bounds.
pub(crate) fn layer_door_overlaps_bounds(
    ecs: &Ecs,
    entity: Entity,
    fallback_bounds: Option<Rect>,
    bounds_union: &[Rect],
) -> bool {
    let bounds = ecs
        .get::<Interactable>(entity)
        .zip(ecs.get::<Transform>(entity))
        .map(|(interactable, transform)| interactable.bounds_at(transform.position))
        .or(fallback_bounds);

    let Some(bounds) = bounds else {
        debug_assert!(false, "LayerDoor missing bounds source for {entity:?}");
        return false;
    };

    bounds_union.iter().any(|zone_bounds| bounds.overlaps(zone_bounds))
}

