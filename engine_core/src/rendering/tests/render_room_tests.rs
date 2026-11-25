use super::*;
use bishop::{material::EmissionPassParams, vec2, vec3, Camera2D, Rect};
use crate::animation::{AsepriteSliceBounds, ClipDef, ClipId};
use crate::ecs::{
    Animation, Cover, CurrentFrame, Glow, GlowSource, Interactable, Pivot, Player, PlayerProxy,
    PointLight, PointLightLayerScope, PointLightStyle, Sprite, SpriteId, Transform,
};
use std::collections::BTreeMap;
use crate::rendering::lighting::point_light_affects_layer;
use crate::rendering::renderable::emission_slice_geometry;
use crate::rendering::room_layer_draw::{GlowSourceDiagnostic, GlowSourceIssueKind};
use crate::rendering::test_support::make_vertical_spillover_fixture;
use crate::rendering::{RoomCompositionContext, RoomRenderState, RoomVisibilityContext};
use crate::worlds::room::RoomVariant;
use crate::worlds::test_utils::make_room;
use crate::worlds::{
    BackRoomLayer, Exit, ExitDirection, InteriorZone, InteriorZoneBounds, InteriorZoneId,
    InteriorZoneScope, LayerCompositionMode, Room, RoomId, RoomLayer, RoomLayers, World, WorldId,
};

fn room_visibility_context<'a>(world: &'a World, room_id: RoomId) -> RoomVisibilityContext<'a> {
    let Some(room) = world.get_room(room_id) else {
        panic!("test fixture is missing room {room_id:?}");
    };
    RoomVisibilityContext {
        world,
        room,
        grid_size: 16.0,
    }
}

fn render_state(current_layer: RoomLayer) -> RoomRenderState {
    RoomRenderState {
        current_layer,
        viewpoint_position: None,
        show_all_back_bounds: false,
    }
}

fn back_view_state(viewpoint_position: Vec2) -> RoomRenderState {
    RoomRenderState {
        current_layer: RoomLayer::Back,
        viewpoint_position: Some(viewpoint_position),
        show_all_back_bounds: false,
    }
}

fn make_composition_test_room(
    composition_mode: LayerCompositionMode,
    zone_scope: InteriorZoneScope,
    interior_zones: Vec<InteriorZone>,
) -> Room {
    Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        size: Vec2::new(8.0, 8.0),
        variants: vec![RoomVariant {
            layers: RoomLayers {
                back: Some(BackRoomLayer {
                    composition_mode,
                    zone_scope,
                    interior_zones,
                }),
            },
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn front_layer_alpha(
    ecs: &Ecs,
    room: &Room,
    entity: Entity,
    bounds: Rect,
    state: RoomRenderState,
    grid_size: f32,
) -> Option<f32> {
    RoomCompositionContext::resolve(room, state, grid_size)
        .front_layer_composition(ecs, entity, Some(bounds))
        .tint()
        .map(|color| color.a)
}

#[test]
fn emission_params_preserve_routed_target_pixel_settings() {
    let room_id = RoomId(1);
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![make_room(Some(1), 0.0, 0.0, 4.0, 4.0)],
        16.0,
    );
    let mut glow = Glow::default();
    glow.color = vec3(0.2, 0.5, 0.9);
    glow.intensity = 3.5;
    glow.halo_spread = 4.0;
    glow.style = PointLightStyle::Pixel;
    glow.pixel_size = 2;
    let mut ecs = Ecs::default();
    ecs.create_entity()
        .with(Transform {
            position: vec2(8.0, 16.0),
            ..Default::default()
        })
        .with(Sprite { sprite: SpriteId(17) })
        .with(glow)
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );
    let camera = Camera2D::new(Vec2::ZERO, vec2(0.5, 0.5));
    let pixel_grid_origin = camera.world_to_screen(Vec2::ZERO, 128.0, 128.0);
    let scale = target_pixels_per_world_unit(&camera, 128.0, 128.0);
    let routed = prepare_routed_glows(
        &camera,
        128.0,
        128.0,
        scale,
        pixel_grid_origin,
        &[RoomLayer::Front],
        &layer_maps,
    );
    let routed = routed.first().expect("visible emitter should be routed");
    let params = EmissionPassParams::from(routed.params);

    assert_eq!(params.color, [0.2, 0.5, 0.9]);
    assert_eq!(params.intensity, 3.5);
    assert_eq!(params.halo_spread, 128.0);
    assert!(params.pixelated);
    assert_eq!(params.pixel_size, 64.0);
    assert_eq!(params.pixel_grid_origin, [64.0, 64.0]);
}

#[test]
fn emission_slice_geometry_uses_frame_cell_and_flip() {
    let bounds = AsepriteSliceBounds {
        x: 1.0,
        y: 2.0,
        w: 3.0,
        h: 4.0,
    };
    let source_origin = vec2(8.0, 8.0);
    let frame_position = vec2(8.0, -1.0);
    let frame_size = vec2(8.0, 8.0);

    let (source, normal_destination) = emission_slice_geometry(
        source_origin,
        frame_position,
        frame_size,
        bounds,
        false,
    );
    let (_, flipped_destination) = emission_slice_geometry(
        source_origin,
        frame_position,
        frame_size,
        bounds,
        true,
    );

    assert_eq!(source, Rect::new(9.0, 10.0, 3.0, 4.0));
    assert_eq!(normal_destination, Rect::new(9.0, 1.0, 3.0, 4.0));
    assert_eq!(flipped_destination, Rect::new(12.0, 1.0, 3.0, 4.0));
}

#[test]
fn glow_culling_keeps_offscreen_source_when_halo_reaches_view() {
    let camera = Camera2D::new(Vec2::ZERO, vec2(0.5, 0.5));
    let source = Rect::new(-2.5, 0.0, 0.25, 0.25);

    assert!(glow_source_visible_with_halo(&camera, source, 20.0, 128.0, 128.0));
    assert!(!glow_source_visible_with_halo(&camera, source, 0.0, 128.0, 128.0));
}

#[test]
fn pixel_light_grid_scales_with_editor_camera_zoom() {
    let zoomed_out = Camera2D::new(Vec2::ZERO, vec2(0.5, 0.5));
    let zoomed_in = Camera2D::new(Vec2::ZERO, vec2(2.0, 2.0));

    assert_eq!(target_pixels_per_world_unit(&zoomed_out, 128.0, 128.0), 32.0);
    assert_eq!(target_pixels_per_world_unit(&zoomed_in, 128.0, 128.0), 128.0);
}

#[test]
fn transform_ron_round_trip_preserves_z() {
    let ron = ron::to_string(&Transform {
        visible: true,
        position: Vec2::new(12.0, 34.0),
        pivot: Pivot::BottomCenter,
        z: 7,
    })
    .unwrap();

    let restored: Transform = ron::from_str(&ron).unwrap();

    assert_eq!(Transform::default().z, 0);
    assert_eq!(restored.z, 7);
}

#[test]
fn collect_interpolated_room_layer_maps_same_z_orders_lower_entity_id_first() {
    let room_id = RoomId(1);
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![make_room(Some(1), 0.0, 0.0, 4.0, 4.0)],
        16.0,
    );
    let mut ecs = Ecs::default();

    let lower = ecs.create_entity()
        .with(Transform {
            z: 3,
            ..Default::default()
        })
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    let higher = ecs.create_entity()
        .with(Transform {
            position: vec2(16.0, 0.0),
            z: 3,
            ..Default::default()
        })
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );
    let entities = layer_maps
        .for_layer(RoomLayer::Front)
        .get(&3)
        .unwrap()
        .items
        .iter()
        .filter_map(|item| match item {
            LayerDrawItem::Entity { entity, .. } => Some(*entity),
            LayerDrawItem::Particle { .. } => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(entities, vec![lower, higher]);
}

#[test]
fn collect_interpolated_room_layer_maps_point_light_uses_interpolated_position_and_own_layer() {
    let room_id = RoomId(1);
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![make_room(Some(1), 0.0, 0.0, 4.0, 4.0)],
        16.0,
    );
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform {
            position: vec2(10.0, 8.0),
            z: 2,
            ..Default::default()
        })
        .with(PointLight {
            offset: vec2(3.0, -1.0),
            layer_scope: PointLightLayerScope::AllVisibleLayers,
            ..Default::default()
        })
        .with_current_room_layer(room_id, RoomLayer::Back)
        .finish();
    let prev_positions = HashMap::from([(entity, vec2(2.0, 4.0))]);

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        0.5,
        Some(&prev_positions),
        None,
    );

    let front_light_count = layer_maps
        .for_layer(RoomLayer::Front)
        .get(&2)
        .map_or(0, |layer| layer.lighting.point_lights.len());
    let back_light = layer_maps
        .for_layer(RoomLayer::Back)
        .get(&2)
        .and_then(|layer| layer.lighting.point_lights.first());

    assert_eq!(front_light_count, 0);
    assert_eq!(back_light.map(|(entity, _, _)| *entity), Some(entity));
    assert_eq!(back_light.map(|(_, _, position)| *position), Some(vec2(9.0, 5.0)));
    assert_eq!(
        back_light.map(|(_, light, _)| light.layer_scope),
        Some(PointLightLayerScope::AllVisibleLayers)
    );
}

#[test]
fn collect_interpolated_room_layer_maps_point_lights_are_sorted_by_entity_id() {
    let room_id = RoomId(1);
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![make_room(Some(1), 0.0, 0.0, 4.0, 4.0)],
        16.0,
    );
    let mut ecs = Ecs::default();
    let first = ecs
        .create_entity()
        .with(Transform {
            z: 2,
            ..Default::default()
        })
        .with(PointLight::default())
        .finish();
    let second = ecs
        .create_entity()
        .with(Transform {
            z: 2,
            ..Default::default()
        })
        .with(PointLight::default())
        .finish();
    ecs.set_current_room_layer(second, room_id, RoomLayer::Front);
    ecs.set_current_room_layer(first, room_id, RoomLayer::Front);

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );
    let lights = layer_maps
        .for_layer(RoomLayer::Front)
        .get(&2)
        .map(|layer| {
            layer
                .lighting
                .point_lights
                .iter()
                .map(|(entity, _, _)| *entity)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    assert_eq!(lights, vec![first, second]);
}

#[test]
fn collect_interpolated_room_layer_maps_emission_resolves_current_slice_frame_and_flip() {
    let room_id = RoomId(1);
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![make_room(Some(1), 0.0, 0.0, 4.0, 4.0)],
        16.0,
    );
    let bounds = AsepriteSliceBounds {
        x: 1.0,
        y: 2.0,
        w: 3.0,
        h: 4.0,
    };
    let mut glow = Glow::default();
    glow.source = GlowSource::AsepriteSlice;
    glow.replace_slice_bounds_for_clip(ClipId::Idle, BTreeMap::from([(3, bounds)]));
    let mut ecs = Ecs::default();
    ecs.create_entity()
        .with(Transform {
            position: vec2(10.0, 8.0),
            z: 2,
            ..Default::default()
        })
        .with(Animation {
            clips: HashMap::from([(
                ClipId::Idle,
                ClipDef {
                    cols: 2,
                    rows: 2,
                    frame_size: vec2(8.0, 8.0),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        })
        .with(CurrentFrame {
            clip_id: ClipId::Idle,
            col: 1,
            row: 1,
            offset: vec2(2.0, -1.0),
            sprite_id: SpriteId(17),
            frame_size: vec2(8.0, 8.0),
            flip_x: true,
        })
        .with(glow)
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );
    let resolved = layer_maps
        .for_layer(RoomLayer::Front)
        .get(&2)
        .and_then(|layer| layer.lighting.glows.first())
        .map(|glow| glow.source.clone())
        .expect("matching imported glow frame should be collected");

    assert_eq!(resolved.clip_id, Some(ClipId::Idle));
    assert_eq!(resolved.frame_index, Some(3));
    assert_eq!(resolved.sprite_id, SpriteId(17));
    assert_eq!(resolved.source_rect, Rect::new(9.0, 10.0, 3.0, 4.0));
    assert_eq!(resolved.destination, Rect::new(12.0, 1.0, 3.0, 4.0));
    assert!(resolved.flip_x);
    assert_eq!(resolved.slice_bounds, Some(bounds));
}

#[test]
fn collect_interpolated_room_layer_maps_emission_uses_animated_player_visual_glow() {
    let room_id = RoomId(1);
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![make_room(Some(1), 0.0, 0.0, 4.0, 4.0)],
        16.0,
    );
    let mut ecs = Ecs::default();
    let player = ecs.create_entity()
        .with(Player)
        .with(Animation {
            clips: HashMap::from([(
                ClipId::Idle,
                ClipDef {
                    cols: 3,
                    rows: 1,
                    frame_size: vec2(8.0, 8.0),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        })
        .with(CurrentFrame {
            clip_id: ClipId::Idle,
            col: 2,
            row: 0,
            sprite_id: SpriteId(17),
            frame_size: vec2(8.0, 8.0),
            ..Default::default()
        })
        .with(Glow::default())
        .finish();
    ecs.create_entity()
        .with(Transform {
            position: vec2(8.0, 8.0),
            ..Default::default()
        })
        .with(PlayerProxy)
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );
    let glows = layer_maps
        .for_layer(RoomLayer::Front)
        .values()
        .flat_map(|layer| &layer.lighting.glows)
        .collect::<Vec<_>>();

    assert_eq!(glows.len(), 1);
    assert_eq!(glows[0].entity, player);
    assert_eq!(glows[0].source.sprite_id, SpriteId(17));
    assert_eq!(glows[0].source.source_rect, Rect::new(16.0, 0.0, 8.0, 8.0));
}

#[test]
fn collect_interpolated_room_layer_maps_emission_requires_current_frame_and_matching_slice() {
    let room_id = RoomId(1);
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![make_room(Some(1), 0.0, 0.0, 4.0, 4.0)],
        16.0,
    );
    let mut glow = Glow::default();
    glow.source = GlowSource::AsepriteSlice;
    glow.replace_slice_bounds_for_clip(
        ClipId::Idle,
        BTreeMap::from([(
            0,
            AsepriteSliceBounds {
                x: 0.0,
                y: 0.0,
                w: 8.0,
                h: 8.0,
            },
        )]),
    );
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(Transform::default())
        .with(Sprite { sprite: SpriteId(17) })
        .with(Animation {
            clips: HashMap::from([(ClipId::Idle, ClipDef::default())]),
            ..Default::default()
        })
        .with(glow)
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );

    assert!(layer_maps
        .for_layer(RoomLayer::Front)
        .values()
        .all(|layer| layer.lighting.glows.is_empty()));
    assert_eq!(
        layer_maps.glow_source_diagnostics,
        vec![GlowSourceDiagnostic {
            entity,
            owner_layer: RoomLayer::Front,
            kind: GlowSourceIssueKind::MissingCurrentFrame,
        }],
    );

    ecs.add_component_to_entity(
        entity,
        CurrentFrame {
            clip_id: ClipId::Idle,
            col: 1,
            row: 0,
            offset: Vec2::ZERO,
            sprite_id: SpriteId(17),
            frame_size: vec2(16.0, 16.0),
            flip_x: false,
        },
    );
    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );

    assert!(layer_maps
        .for_layer(RoomLayer::Front)
        .values()
        .all(|layer| layer.lighting.glows.is_empty()));
    assert_eq!(
        layer_maps.glow_source_diagnostics,
        vec![GlowSourceDiagnostic {
            entity,
            owner_layer: RoomLayer::Front,
            kind: GlowSourceIssueKind::MissingSliceBounds,
        }],
    );
}

#[test]
fn point_light_layer_scope_respects_visible_layers_and_owner_layer() {
    let owner_layer = RoomLayer::Back;
    let visible = visible_layers_for_state(
        &RoomLayers {
            back: Some(BackRoomLayer {
                composition_mode: LayerCompositionMode::DollsHouse,
                ..Default::default()
            }),
        },
        render_state(RoomLayer::Front),
    );
    let owning_layer_light = PointLight::default();
    let all_visible_light = PointLight {
        layer_scope: PointLightLayerScope::AllVisibleLayers,
        ..Default::default()
    };

    assert!(point_light_affects_layer(
        &owning_layer_light,
        owner_layer,
        RoomLayer::Back,
        &visible.ordered_layers,
    ));
    assert!(!point_light_affects_layer(
        &owning_layer_light,
        owner_layer,
        RoomLayer::Front,
        &visible.ordered_layers,
    ));
    assert!(point_light_affects_layer(
        &all_visible_light,
        owner_layer,
        RoomLayer::Front,
        &visible.ordered_layers,
    ));

    let hidden_back = visible_layers_for_state(
        &RoomLayers {
            back: Some(BackRoomLayer::default()),
        },
        render_state(RoomLayer::Front),
    );
    assert!(!point_light_affects_layer(
        &all_visible_light,
        owner_layer,
        RoomLayer::Back,
        &hidden_back.ordered_layers,
    ));
}

#[test]
fn current_layer_front_when_hidden_then_renders_front_only() {
    let visible = visible_layers_for_state(
        &RoomLayers {
            back: Some(BackRoomLayer::default()),
        },
        render_state(RoomLayer::Front),
    );

    assert_eq!(visible.ordered_layers, vec![RoomLayer::Front]);
}

#[test]
fn current_layer_back_when_hidden_then_renders_back_only() {
    let visible = visible_layers_for_state(
        &RoomLayers {
            back: Some(BackRoomLayer::default()),
        },
        render_state(RoomLayer::Back),
    );

    assert_eq!(visible.ordered_layers, vec![RoomLayer::Back]);
}

#[test]
fn current_layer_front_when_dolls_house_then_renders_back_then_front() {
    let visible = visible_layers_for_state(
        &RoomLayers {
            back: Some(BackRoomLayer {
                composition_mode: LayerCompositionMode::DollsHouse,
                zone_scope: InteriorZoneScope::Occupied,
                ..Default::default()
            }),
        },
        render_state(RoomLayer::Front),
    );

    assert_eq!(visible.ordered_layers, vec![RoomLayer::Back, RoomLayer::Front]);
}

#[test]
fn current_layer_back_when_dolls_house_then_renders_back_then_front() {
    let visible = visible_layers_for_state(
        &RoomLayers {
            back: Some(BackRoomLayer {
                composition_mode: LayerCompositionMode::DollsHouse,
                zone_scope: InteriorZoneScope::Occupied,
                ..Default::default()
            }),
        },
        render_state(RoomLayer::Back),
    );

    assert_eq!(visible.ordered_layers, vec![RoomLayer::Back, RoomLayer::Front]);
}

#[test]
fn back_view_when_show_all_back_bounds_then_other_zone_bounds_are_visible() {
    let room = make_composition_test_room(
        LayerCompositionMode::DollsHouse,
        InteriorZoneScope::Occupied,
        vec![
            InteriorZone {
                id: InteriorZoneId(1),
                bounds: InteriorZoneBounds::new(0, 0, 32, 32),
            },
            InteriorZone {
                id: InteriorZoneId(2),
                bounds: InteriorZoneBounds::new(64, 0, 32, 32),
            },
        ],
    );
    let composition = RoomCompositionContext::resolve(
        &room,
        RoomRenderState {
            current_layer: RoomLayer::Back,
            viewpoint_position: Some(Vec2::new(8.0, 8.0)),
            show_all_back_bounds: true,
        },
        16.0,
    );

    assert!(composition.back_layer_bounds_visible(Rect::new(0.0, 0.0, 32.0, 32.0)));
    assert!(composition.back_layer_bounds_visible(Rect::new(64.0, 0.0, 32.0, 32.0)));
}

#[test]
fn back_view_when_show_all_back_bounds_then_content_outside_zones_is_visible() {
    let room = make_composition_test_room(
        LayerCompositionMode::DollsHouse,
        InteriorZoneScope::Occupied,
        vec![InteriorZone {
            id: InteriorZoneId(1),
            bounds: InteriorZoneBounds::new(0, 0, 32, 32),
        }],
    );
    let composition = RoomCompositionContext::resolve(
        &room,
        RoomRenderState {
            current_layer: RoomLayer::Back,
            viewpoint_position: Some(Vec2::new(8.0, 8.0)),
            show_all_back_bounds: true,
        },
        16.0,
    );

    assert!(composition.back_layer_bounds_visible(Rect::new(96.0, 0.0, 16.0, 16.0)));
}

#[test]
fn back_view_when_zone_scope_is_occupied_then_other_zone_bounds_are_not_visible() {
    let room = make_composition_test_room(
        LayerCompositionMode::DollsHouse,
        InteriorZoneScope::Occupied,
        vec![
            InteriorZone {
                id: InteriorZoneId(1),
                bounds: InteriorZoneBounds::new(0, 0, 32, 32),
            },
            InteriorZone {
                id: InteriorZoneId(2),
                bounds: InteriorZoneBounds::new(64, 0, 32, 32),
            },
        ],
    );
    let composition = RoomCompositionContext::resolve(
        &room,
        back_view_state(Vec2::new(8.0, 8.0)),
        16.0,
    );

    assert!(composition.back_layer_bounds_visible(Rect::new(0.0, 0.0, 32.0, 32.0)));
    assert!(!composition.back_layer_bounds_visible(Rect::new(64.0, 0.0, 32.0, 32.0)));
}

#[test]
fn hidden_back_view_when_zone_scope_is_all_then_other_zone_layer_door_stays_visible() {
    let room = make_composition_test_room(
        LayerCompositionMode::Hidden,
        InteriorZoneScope::All,
        vec![
            InteriorZone {
                id: InteriorZoneId(1),
                bounds: InteriorZoneBounds::new(0, 0, 32, 32),
            },
            InteriorZone {
                id: InteriorZoneId(2),
                bounds: InteriorZoneBounds::new(64, 0, 32, 32),
            },
        ],
    );
    let composition = RoomCompositionContext::resolve(
        &room,
        back_view_state(Vec2::new(8.0, 8.0)),
        16.0,
    );
    assert!(composition.back_layer_bounds_visible(Rect::new(64.0, 0.0, 32.0, 32.0)));

    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(LayerDoor {
            usable: true,
            alpha: 0.25,
        })
        .with(Transform {
            position: Vec2::new(72.0, 8.0),
            ..Default::default()
        })
        .with(Interactable::rect(Vec2::ZERO, Vec2::new(16.0, 16.0)))
        .with_current_room(room.id)
        .finish();

    assert_eq!(
        front_layer_alpha(
            &ecs,
            &room,
            entity,
            Rect::new(64.0, 0.0, 32.0, 32.0),
            back_view_state(Vec2::new(8.0, 8.0)),
            16.0,
        ),
        Some(0.25),
    );
}

#[test]
fn dolls_house_back_view_hides_cover_over_active_zone() {
    let room = make_composition_test_room(
        LayerCompositionMode::DollsHouse,
        InteriorZoneScope::Occupied,
        vec![
            InteriorZone {
                id: InteriorZoneId(1),
                bounds: InteriorZoneBounds::new(0, 0, 32, 32),
            },
            InteriorZone {
                id: InteriorZoneId(2),
                bounds: InteriorZoneBounds::new(64, 0, 32, 32),
            },
        ],
    );
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(Cover::hide())
        .with_current_room(room.id)
        .finish();

    assert_eq!(
        front_layer_alpha(
            &ecs,
            &room,
            entity,
            Rect::new(0.0, 0.0, 32.0, 32.0),
            back_view_state(Vec2::new(8.0, 8.0)),
            16.0,
        ),
        None,
    );
}

#[test]
fn dolls_house_back_view_keeps_other_zone_cover_opaque() {
    let room = make_composition_test_room(
        LayerCompositionMode::DollsHouse,
        InteriorZoneScope::Occupied,
        vec![
            InteriorZone {
                id: InteriorZoneId(1),
                bounds: InteriorZoneBounds::new(0, 0, 32, 32),
            },
            InteriorZone {
                id: InteriorZoneId(2),
                bounds: InteriorZoneBounds::new(64, 0, 32, 32),
            },
        ],
    );
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(Cover::hide())
        .with_current_room(room.id)
        .finish();

    assert_eq!(
        front_layer_alpha(
            &ecs,
            &room,
            entity,
            Rect::new(64.0, 0.0, 32.0, 32.0),
            back_view_state(Vec2::new(8.0, 8.0)),
            16.0,
        ),
        Some(1.0),
    );
}

#[test]
fn dolls_house_back_view_fades_cover_over_active_zone() {
    let room = make_composition_test_room(
        LayerCompositionMode::DollsHouse,
        InteriorZoneScope::Occupied,
        vec![InteriorZone {
            id: InteriorZoneId(1),
            bounds: InteriorZoneBounds::new(0, 0, 32, 32),
        }],
    );
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(Cover::fade(0.35))
        .with_current_room(room.id)
        .finish();

    assert_eq!(
        front_layer_alpha(
            &ecs,
            &room,
            entity,
            Rect::new(0.0, 0.0, 32.0, 32.0),
            back_view_state(Vec2::new(8.0, 8.0)),
            16.0,
        ),
        Some(0.35),
    );
}

#[test]
fn hidden_back_view_keeps_layer_door_visible_with_ghost_alpha() {
    let room = make_composition_test_room(
        LayerCompositionMode::Hidden,
        InteriorZoneScope::Occupied,
        vec![],
    );
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(LayerDoor {
            usable: true,
            alpha: 0.25,
        })
        .with_current_room(room.id)
        .finish();

    assert_eq!(
        front_layer_alpha(
            &ecs,
            &room,
            entity,
            Rect::new(0.0, 0.0, 16.0, 16.0),
            back_view_state(Vec2::new(8.0, 8.0)),
            16.0,
        ),
        Some(0.25),
    );
}

#[test]
fn hidden_back_view_hides_layer_door_outside_active_zone() {
    let room = make_composition_test_room(
        LayerCompositionMode::Hidden,
        InteriorZoneScope::Occupied,
        vec![
            InteriorZone {
                id: InteriorZoneId(1),
                bounds: InteriorZoneBounds::new(0, 0, 32, 32),
            },
            InteriorZone {
                id: InteriorZoneId(2),
                bounds: InteriorZoneBounds::new(64, 0, 32, 32),
            },
        ],
    );
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(LayerDoor {
            usable: true,
            alpha: 0.25,
        })
        .with(Transform {
            position: Vec2::new(72.0, 8.0),
            ..Default::default()
        })
        .with(Interactable::rect(Vec2::ZERO, Vec2::new(16.0, 16.0)))
        .with_current_room(room.id)
        .finish();

    assert_eq!(
        front_layer_alpha(
            &ecs,
            &room,
            entity,
            Rect::new(64.0, 0.0, 32.0, 32.0),
            back_view_state(Vec2::new(8.0, 8.0)),
            16.0,
        ),
        None,
    );
}

#[test]
fn dolls_house_back_view_keeps_other_zone_layer_door_opaque() {
    let room = make_composition_test_room(
        LayerCompositionMode::DollsHouse,
        InteriorZoneScope::Occupied,
        vec![
            InteriorZone {
                id: InteriorZoneId(1),
                bounds: InteriorZoneBounds::new(0, 0, 32, 32),
            },
            InteriorZone {
                id: InteriorZoneId(2),
                bounds: InteriorZoneBounds::new(64, 0, 32, 32),
            },
        ],
    );
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(LayerDoor {
            usable: true,
            alpha: 0.25,
        })
        .with(Transform {
            position: Vec2::new(72.0, 8.0),
            ..Default::default()
        })
        .with(Interactable::rect(Vec2::ZERO, Vec2::new(16.0, 16.0)))
        .with_current_room(room.id)
        .finish();

    assert_eq!(
        front_layer_alpha(
            &ecs,
            &room,
            entity,
            Rect::new(64.0, 0.0, 32.0, 32.0),
            back_view_state(Vec2::new(8.0, 8.0)),
            16.0,
        ),
        Some(1.0),
    );
}

#[test]
fn collect_interpolated_room_layer_maps_front_skips_entities_outside_the_room_index() {
    let room_id = RoomId(1);
    let other_room = RoomId(2);
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![
            make_room(Some(1), 0.0, 0.0, 4.0, 4.0),
            make_room(Some(2), 80.0, 0.0, 4.0, 4.0),
        ],
        16.0,
    );
    let mut ecs = Ecs::default();

    let visible = ecs.create_entity()
        .with(Transform::default())
        .with_current_room(room_id)
        .finish();

    ecs.create_entity()
        .with(Transform::default())
        .with_current_room(other_room)
        .finish();

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );
    let layers = layer_maps.for_layer(RoomLayer::Front);

    assert!(layers
        .values()
        .flat_map(|layer| &layer.items)
        .any(|item| matches!(item, LayerDrawItem::Entity { entity, .. } if *entity == visible)));
}

#[test]
fn cross_room_visibility_collect_interpolated_room_layer_maps_front_excludes_other_room_entity_at_non_exit_boundary() {
    let (world, ecs, entity, room_id) = make_vertical_spillover_fixture(vec2(32.0, 64.0), None);

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );
    let layers = layer_maps.for_layer(RoomLayer::Front);

    assert!(!layers.values().flat_map(|layer| &layer.items).any(|item| matches!(item, LayerDrawItem::Entity { entity: id, .. } if *id == entity)));
}

#[test]
fn cross_room_visibility_collect_interpolated_room_layer_maps_front_includes_other_room_entity_through_exit_cell() {
    let (world, ecs, entity, room_id) = make_vertical_spillover_fixture(
        vec2(32.0, 64.0),
        Some(Exit {
            position: vec2(1.0, 4.0),
            direction: ExitDirection::Down,
            layer: RoomLayer::Front,
            target_room_id: Some(RoomId(2)),
        }),
    );

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );
    let layers = layer_maps.for_layer(RoomLayer::Front);

    assert!(layers.values().flat_map(|layer| &layer.items).any(|item| matches!(item, LayerDrawItem::Entity { entity: id, .. } if *id == entity)));
}

#[test]
fn cross_room_visibility_collect_interpolated_room_layer_maps_front_excludes_other_room_entity_once_fully_outside() {
    let (world, ecs, entity, room_id) = make_vertical_spillover_fixture(vec2(32.0, 96.0), None);

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        None,
    );
    let layers = layer_maps.for_layer(RoomLayer::Front);

    assert!(!layers.values().flat_map(|layer| &layer.items).any(|item| matches!(item, LayerDrawItem::Entity { entity: id, .. } if *id == entity)));
}
