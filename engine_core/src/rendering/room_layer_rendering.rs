use crate::ecs::{Entity, PointLightLayerScope};
use crate::worlds::RoomLayer;
use bishop::prelude::PointLightPassParams;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayerLightingBuffer {
    A,
    B,
}

pub(crate) fn alternate_lighting_buffer(source: LayerLightingBuffer) -> LayerLightingBuffer {
    match source {
        LayerLightingBuffer::A => LayerLightingBuffer::B,
        LayerLightingBuffer::B => LayerLightingBuffer::A,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LightingFailure {
    LayerTargetAllocation,
    FinalTargetAllocation,
    LightingPass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LightingFallback {
    CompositeUnlitContributions,
    DirectUnlitRoomRender,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LightingFailureOutcome {
    pub(crate) fallback: LightingFallback,
    pub(crate) diagnostic_code: &'static str,
}

pub(crate) fn fallback_for_lighting_failure(failure: LightingFailure) -> LightingFailureOutcome {
    match failure {
        LightingFailure::LayerTargetAllocation => LightingFailureOutcome {
            fallback: LightingFallback::DirectUnlitRoomRender,
            diagnostic_code: "lighting_layer_target_allocation_failed",
        },
        LightingFailure::FinalTargetAllocation => LightingFailureOutcome {
            fallback: LightingFallback::DirectUnlitRoomRender,
            diagnostic_code: "lighting_final_target_allocation_failed",
        },
        LightingFailure::LightingPass => LightingFailureOutcome {
            fallback: LightingFallback::CompositeUnlitContributions,
            diagnostic_code: "lighting_pass_failed",
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RoomLightingContribution {
    Background,
    Layer(RoomLayer),
    DoorGhosts,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RoutedPointLight {
    pub(crate) entity: Entity,
    pub(crate) owner_layer: RoomLayer,
    pub(crate) scope: PointLightLayerScope,
    pub(crate) params: PointLightPassParams,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ContributionLightingPlan {
    pub(crate) contribution: RoomLightingContribution,
    pub(crate) point_lights: Vec<RoutedPointLight>,
}

pub(crate) fn build_room_lighting_plan(
    visible_layers: &[RoomLayer],
    point_lights: &[RoutedPointLight],
    include_door_ghosts: bool,
) -> Vec<ContributionLightingPlan> {
    let mut plan = Vec::with_capacity(visible_layers.len() + 1);
    plan.push(ContributionLightingPlan {
        contribution: RoomLightingContribution::Background,
        point_lights: Vec::new(),
    });

    for &target_layer in visible_layers {
        let mut lights = point_lights
            .iter()
            .filter(|light| {
                visible_layers.contains(&light.owner_layer)
                    && match light.scope {
                        PointLightLayerScope::OwningLayer => light.owner_layer == target_layer,
                        PointLightLayerScope::AllVisibleLayers => true,
                    }
            })
            .copied()
            .collect::<Vec<_>>();
        lights.sort_by_key(|light| light.entity);
        plan.push(ContributionLightingPlan {
            contribution: RoomLightingContribution::Layer(target_layer),
            point_lights: lights,
        });
    }

    if include_door_ghosts {
        plan.push(ContributionLightingPlan {
            contribution: RoomLightingContribution::DoorGhosts,
            point_lights: Vec::new(),
        });
    }

    plan
}

#[cfg(test)]
#[path = "tests/room_layer_rendering_tests.rs"]
mod tests;
