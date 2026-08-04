use super::*;
use crate::rendering::test_support::make_vertical_spillover_fixture;
use crate::rendering::{RoomCompositionContext, RoomRenderState};
use crate::worlds::room::RoomVariant;
use crate::worlds::test_utils::make_room;
use crate::worlds::{
    BackRoomLayer, Exit, ExitDirection, InteriorZone, InteriorZoneBounds, InteriorZoneId,
    LayerCompositionMode, RoomLayer, RoomLayers, WorldId,
};

fn render_state(current_layer: RoomLayer) -> RoomRenderState {
    RoomRenderState {
        current_layer,
        viewpoint_position: None,
    }
}

fn back_view_state(viewpoint_position: Vec2) -> RoomRenderState {
    RoomRenderState {
        current_layer: RoomLayer::Back,
        viewpoint_position: Some(viewpoint_position),
    }
}

fn make_composition_test_room(
    composition_mode: LayerCompositionMode,
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
                ..Default::default()
            }),
        },
        render_state(RoomLayer::Back),
    );

    assert_eq!(visible.ordered_layers, vec![RoomLayer::Back, RoomLayer::Front]);
}

#[test]
fn dolls_house_back_view_hides_cover_over_active_zone() {
    let room = make_composition_test_room(
        LayerCompositionMode::DollsHouse,
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
    let room = make_composition_test_room(LayerCompositionMode::Hidden, vec![]);
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
        &world,
        world.get_room(room_id).unwrap(),
        &SpriteManager::default(),
        1.0,
        None,
        16.0,
    );
    let layers = layer_maps.for_layer(RoomLayer::Front);

    assert!(layers
        .values()
        .flat_map(|layer| layer.entities.iter())
        .any(|(entity, _)| *entity == visible));
}

#[test]
fn cross_room_visibility_collect_interpolated_room_layer_maps_front_excludes_other_room_entity_at_non_exit_boundary() {
    let (world, ecs, entity, room_id) = make_vertical_spillover_fixture(vec2(32.0, 64.0), None);

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &world,
        world.get_room(room_id).unwrap(),
        &SpriteManager::default(),
        1.0,
        None,
        16.0,
    );
    let layers = layer_maps.for_layer(RoomLayer::Front);

    assert!(!layers.values().flat_map(|layer| layer.entities.iter()).any(|(id, _)| *id == entity));
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
        &world,
        world.get_room(room_id).unwrap(),
        &SpriteManager::default(),
        1.0,
        None,
        16.0,
    );
    let layers = layer_maps.for_layer(RoomLayer::Front);

    assert!(layers.values().flat_map(|layer| layer.entities.iter()).any(|(id, _)| *id == entity));
}

#[test]
fn cross_room_visibility_collect_interpolated_room_layer_maps_front_excludes_other_room_entity_once_fully_outside() {
    let (world, ecs, entity, room_id) = make_vertical_spillover_fixture(vec2(32.0, 96.0), None);

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &world,
        world.get_room(room_id).unwrap(),
        &SpriteManager::default(),
        1.0,
        None,
        16.0,
    );
    let layers = layer_maps.for_layer(RoomLayer::Front);

    assert!(!layers.values().flat_map(|layer| layer.entities.iter()).any(|(id, _)| *id == entity));
}
