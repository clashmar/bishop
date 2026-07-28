use crate::inspector_module;
use bishop::prelude::Rect;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Semantic cover behavior used by helper APIs and tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverMode {
    Hide,
    Fade { alpha: f32 },
}

/// Shared cover behavior component for entities and tile definitions.
#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Reflect, PartialEq, Default)]
#[serde(default)]
pub struct Cover {
    /// When true, front content is hidden instead of faded while the back layer is active.
    pub hide: bool,
    /// Alpha applied when `hide == false`.
    pub fade_alpha: f32,
}

inspector_module!(Cover);

impl Cover {
    /// Creates a fully hidden cover.
    pub fn hide() -> Self {
        Self {
            hide: true,
            fade_alpha: 0.0,
        }
    }

    /// Creates a fading cover with the supplied alpha.
    pub fn fade(alpha: f32) -> Self {
        Self {
            hide: false,
            fade_alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Returns the semantic mode represented by this component.
    pub fn mode(self) -> CoverMode {
        if self.hide {
            CoverMode::Hide
        } else {
            CoverMode::Fade {
                alpha: self.fade_alpha,
            }
        }
    }
}

/// Returns true when a cover visual overlaps any active bounds.
pub(crate) fn cover_overlaps_bounds(bounds: Rect, bounds_union: &[Rect]) -> bool {
    bounds_union.iter().any(|zone_bounds| bounds.overlaps(zone_bounds))
}
