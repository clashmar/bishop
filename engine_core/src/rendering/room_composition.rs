use crate::ecs::components::cover::cover_overlaps_bounds;
use crate::ecs::components::layer_door::layer_door_overlaps_bounds;
use crate::ecs::*;
use crate::worlds::room::Room;
use crate::worlds::{LayerCompositionMode, RoomId, RoomLayer};
use bishop::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomRenderState {
    pub current_layer: RoomLayer,
    pub viewpoint_position: Option<Vec2>,
    pub show_all_back_bounds: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FrontLayerComposition {
    Hidden,
    Opaque,
    Alpha(f32),
}

impl FrontLayerComposition {
    pub(crate) fn tint(self) -> Option<Color> {
        match self {
            Self::Hidden => None,
            Self::Opaque => Some(Color::WHITE),
            Self::Alpha(alpha) => Some(Color::new(1.0, 1.0, 1.0, alpha)),
        }
    }
}

/// Room-wide composition state resolved once before drawing front-layer content.
pub struct RoomCompositionContext {
    pub(crate) room_id: RoomId,
    pub(crate) room_position: Vec2,
    pub(crate) grid_size: f32,
    current_layer: RoomLayer,
    composition_mode: Option<LayerCompositionMode>,
    active_back_bounds: Vec<Rect>,
}

impl RoomCompositionContext {
    pub fn resolve(room: &Room, state: RoomRenderState, grid_size: f32) -> Self {
        let layers = &room.current_variant().layers;
        let room_bounds = room.world_rect(grid_size);

        let active_back_bounds = if state.show_all_back_bounds {
            vec![room_bounds]
        } else {
            layers.active_back_bounds(room_bounds, state.current_layer, state.viewpoint_position)
        };

        Self {
            room_id: room.id,
            room_position: room.position,
            grid_size,
            current_layer: state.current_layer,
            composition_mode: layers.back.as_ref().map(|back| back.composition_mode),
            active_back_bounds,
        }
    }

    pub(crate) fn should_draw_hidden_back_layer_door_ghosts(&self) -> bool {
        self.current_layer == RoomLayer::Back
            && self.composition_mode == Some(LayerCompositionMode::Hidden)
    }

    pub(crate) fn back_layer_bounds_visible(&self, bounds: Rect) -> bool {
        if self.current_layer != RoomLayer::Back {
            return true;
        }

        if self.active_back_bounds.is_empty() {
            return false;
        }

        self.active_back_bounds
            .iter()
            .any(|active_bounds| active_bounds.overlaps(&bounds))
    }

    /// Returns the composition to use for one front-layer drawable in the current room view.
    pub(crate) fn front_layer_composition(
        &self,
        ecs: &Ecs,
        entity: Entity,
        visual_bounds: Option<Rect>,
    ) -> FrontLayerComposition {
        if self.current_layer != RoomLayer::Back {
            return FrontLayerComposition::Opaque;
        }

        let Some(composition_mode) = self.composition_mode else {
            return FrontLayerComposition::Opaque;
        };

        if let Some(layer_door) = ecs.get::<LayerDoor>(entity) {
            if layer_door_overlaps_bounds(ecs, entity, visual_bounds, &self.active_back_bounds) {
                return FrontLayerComposition::Alpha(layer_door.alpha.clamp(0.0, 1.0));
            }

            return match composition_mode {
                LayerCompositionMode::Hidden => FrontLayerComposition::Hidden,
                LayerCompositionMode::DollsHouse => FrontLayerComposition::Opaque,
            };
        }

        match composition_mode {
            LayerCompositionMode::Hidden => FrontLayerComposition::Hidden,
            LayerCompositionMode::DollsHouse => {
                let Some(cover) = ecs.get::<Cover>(entity).copied() else {
                    return FrontLayerComposition::Opaque;
                };
                let Some(bounds) = visual_bounds else {
                    debug_assert!(false, "Cover missing visual bounds for {entity:?}");
                    return FrontLayerComposition::Opaque;
                };
                if !cover_overlaps_bounds(bounds, &self.active_back_bounds) {
                    return FrontLayerComposition::Opaque;
                }

                match cover.mode() {
                    CoverMode::Hide => FrontLayerComposition::Hidden,
                    CoverMode::Fade { alpha } => {
                        FrontLayerComposition::Alpha(alpha.clamp(0.0, 1.0))
                    }
                }
            }
        }
    }
}
