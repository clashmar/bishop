use crate::ecs::{Entity, PointLightLayerScope};
use crate::rendering::lighting::{GlowPassParams, GlowVisual, light_scope_affects_layer};
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
pub(crate) struct RoutedGlow {
    pub(crate) entity: Entity,
    pub(crate) owner_layer: RoomLayer,
    pub(crate) scope: PointLightLayerScope,
    pub(crate) params: GlowPassParams,
    pub(crate) source: GlowVisual,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ContributionLightingPlan {
    pub(crate) contribution: RoomLightingContribution,
    pub(crate) point_lights: Vec<RoutedPointLight>,
    pub(crate) glows: Vec<RoutedGlow>,
}

pub(crate) fn build_room_lighting_plan(
    visible_layers: &[RoomLayer],
    point_lights: &[RoutedPointLight],
    glows: &[RoutedGlow],
    include_door_ghosts: bool,
) -> Vec<ContributionLightingPlan> {
    let mut visible_lights = point_lights
        .iter()
        .filter(|light| visible_layers.contains(&light.owner_layer))
        .copied()
        .collect::<Vec<_>>();
    visible_lights.sort_by_key(|light| light.entity);

    let mut visible_glows = glows
        .iter()
        .filter(|glow| visible_layers.contains(&glow.owner_layer))
        .cloned()
        .collect::<Vec<_>>();
    visible_glows.sort_by_key(|glow| glow.entity);

    let mut plan = Vec::with_capacity(visible_layers.len() + 1);
    // The room background has no Front/Back owner layer, so all visible-room lights illuminate it
    plan.push(ContributionLightingPlan {
        contribution: RoomLightingContribution::Background,
        point_lights: visible_lights.clone(),
        glows: visible_glows.clone(),
    });

    for &target_layer in visible_layers {
        let lights = visible_lights
            .iter()
            .filter(|light| {
                light_scope_affects_layer(
                    light.scope,
                    light.owner_layer,
                    target_layer,
                    visible_layers,
                )
            })
            .copied()
            .collect::<Vec<_>>();
        let routed_glows = visible_glows
            .iter()
            .filter(|glow| {
                light_scope_affects_layer(
                    glow.scope,
                    glow.owner_layer,
                    target_layer,
                    visible_layers,
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        plan.push(ContributionLightingPlan {
            contribution: RoomLightingContribution::Layer(target_layer),
            point_lights: lights,
            glows: routed_glows,
        });
    }

    if include_door_ghosts {
        plan.push(ContributionLightingPlan {
            contribution: RoomLightingContribution::DoorGhosts,
            point_lights: Vec::new(),
            glows: Vec::new(),
        });
    }

    plan
}

#[cfg(test)]
#[path = "tests/room_layer_rendering_tests.rs"]
mod tests;
