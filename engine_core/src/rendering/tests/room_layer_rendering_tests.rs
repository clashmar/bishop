use super::{
    LayerLightingBuffer, LightingFailure, LightingFallback, RoomLightingContribution,
    RoutedPointLight, alternate_lighting_buffer, build_room_lighting_plan,
    fallback_for_lighting_failure,
};
use crate::ecs::{Ecs, Entity, PointLightLayerScope};
use crate::worlds::RoomLayer;
use bishop::prelude::PointLightPassParams;

fn new_entity(ecs: &mut Ecs) -> Entity {
    ecs.create_entity().finish()
}

fn routed_light(entity: Entity, owner_layer: RoomLayer, scope: PointLightLayerScope) -> RoutedPointLight {
    RoutedPointLight {
        entity,
        owner_layer,
        scope,
        params: PointLightPassParams {
            center: [0.0, 0.0],
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            radius: 8.0,
            attenuation: 1.0,
            pixelated: false,
            pixel_size: 1.0,
            pixel_grid_origin: [0.0, 0.0],
        },
    }
}

#[test]
fn room_lighting_plan_keeps_background_and_visible_layer_order() {
    let visible_layers = [RoomLayer::Back, RoomLayer::Front];
    let plan = build_room_lighting_plan(&visible_layers, &[], false);

    assert_eq!(
        plan.iter().map(|entry| entry.contribution).collect::<Vec<_>>(),
        vec![
            RoomLightingContribution::Background,
            RoomLightingContribution::Layer(RoomLayer::Back),
            RoomLightingContribution::Layer(RoomLayer::Front),
        ],
    );
    assert!(plan.iter().all(|entry| entry.point_lights.is_empty()));
}

#[test]
fn room_lighting_plan_routes_visible_lights_to_flat_background() {
    let mut ecs = Ecs::default();
    let back_owning = new_entity(&mut ecs);
    let back_cross_layer = new_entity(&mut ecs);
    let front_owning = new_entity(&mut ecs);
    let lights = [
        routed_light(back_owning, RoomLayer::Back, PointLightLayerScope::OwningLayer),
        routed_light(back_cross_layer, RoomLayer::Back, PointLightLayerScope::AllVisibleLayers),
        routed_light(front_owning, RoomLayer::Front, PointLightLayerScope::OwningLayer),
    ];

    let plan = build_room_lighting_plan(&[RoomLayer::Back, RoomLayer::Front], &lights, false);
    assert_eq!(
        plan[0].point_lights.iter().map(|light| light.entity).collect::<Vec<_>>(),
        vec![back_owning, back_cross_layer, front_owning],
        "the unlayered background receives all lights from visible owner layers",
    );

    let front_only_plan = build_room_lighting_plan(&[RoomLayer::Front], &lights, false);
    assert_eq!(
        front_only_plan[0].point_lights.iter().map(|light| light.entity).collect::<Vec<_>>(),
        vec![front_owning],
        "lights owned by hidden layers do not illuminate the background",
    );
    assert_eq!(
        front_only_plan[1].point_lights.iter().map(|light| light.entity).collect::<Vec<_>>(),
        vec![front_owning],
        "the owning-layer scope remains unchanged for the front layer",
    );
}

#[test]
fn room_lighting_plan_routes_lights_only_to_selected_visible_layers() {
    let mut ecs = Ecs::default();
    let back_only = new_entity(&mut ecs);
    let cross_layer = new_entity(&mut ecs);
    let hidden_owner = new_entity(&mut ecs);
    let visible_layers = [RoomLayer::Back, RoomLayer::Front];
    let lights = [
        routed_light(back_only, RoomLayer::Back, PointLightLayerScope::OwningLayer),
        routed_light(cross_layer, RoomLayer::Back, PointLightLayerScope::AllVisibleLayers),
        routed_light(hidden_owner, RoomLayer::Back, PointLightLayerScope::AllVisibleLayers),
    ];

    let plan = build_room_lighting_plan(&visible_layers, &lights[..2], false);

    assert_eq!(
        plan[1].point_lights.iter().map(|light| light.entity).collect::<Vec<_>>(),
        vec![back_only, cross_layer],
        "back receives its owning-layer light and the explicitly cross-layer light",
    );
    assert_eq!(
        plan[2].point_lights.iter().map(|light| light.entity).collect::<Vec<_>>(),
        vec![cross_layer],
        "front receives only the explicitly cross-layer light",
    );

    let hidden_back_plan = build_room_lighting_plan(&[RoomLayer::Front], &lights, false);
    assert_eq!(hidden_back_plan.len(), 2);
    assert!(hidden_back_plan[1].point_lights.is_empty());
}

#[test]
fn editor_door_ghosts_are_planned_as_an_unlit_top_overlay() {
    let plan = build_room_lighting_plan(&[RoomLayer::Back], &[], true);

    assert_eq!(
        plan.last().map(|entry| entry.contribution),
        Some(RoomLightingContribution::DoorGhosts),
    );
    assert!(plan.last().is_some_and(|entry| entry.point_lights.is_empty()));
}

#[test]
fn point_light_passes_alternate_distinct_per_layer_buffers() {
    let mut source = LayerLightingBuffer::A;

    for _ in 0..8 {
        let destination = alternate_lighting_buffer(source);
        assert_ne!(source, destination);
        source = destination;
    }
}

#[test]
fn failed_pass_uses_complete_unlit_layer_composite_with_diagnostic() {
    let outcome = fallback_for_lighting_failure(LightingFailure::LightingPass);

    assert_eq!(outcome.fallback, LightingFallback::CompositeUnlitContributions);
    assert!(!outcome.diagnostic_code.is_empty());
}

#[test]
fn failed_target_allocation_uses_direct_unlit_room_fallback_with_diagnostic() {
    for failure in [
        LightingFailure::LayerTargetAllocation,
        LightingFailure::FinalTargetAllocation,
    ] {
        let outcome = fallback_for_lighting_failure(failure);

        assert_eq!(outcome.fallback, LightingFallback::DirectUnlitRoomRender);
        assert!(!outcome.diagnostic_code.is_empty());
    }
}

#[test]
fn room_lighting_plan_sorts_light_ids_independently_of_input_order() {
    let mut ecs = Ecs::default();
    let first = new_entity(&mut ecs);
    let second = new_entity(&mut ecs);
    let lights = [
        routed_light(second, RoomLayer::Front, PointLightLayerScope::OwningLayer),
        routed_light(first, RoomLayer::Front, PointLightLayerScope::OwningLayer),
    ];

    let plan = build_room_lighting_plan(&[RoomLayer::Front], &lights, false);

    assert_eq!(
        plan[1].point_lights.iter().map(|light| light.entity).collect::<Vec<_>>(),
        vec![first, second],
    );
}
