use super::*;
use crate::ecs::components::{ParticleEmitter, ParticleEmitterConfig, Transform};
use crate::rendering::{ParticleSystem, RoomVisibilityContext};
use crate::rendering::test_support::make_vertical_spillover_fixture;
use crate::worlds::room::RoomId;
use crate::worlds::test_utils::make_room;
use crate::worlds::{Exit, ExitDirection, RoomLayer, World, WorldId};
use bishop::types::Vec2;

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

fn make_room_world() -> (World, RoomId) {
    let room_id = RoomId(1);
    let world = World::from_rooms(
        WorldId(0),
        String::new(),
        vec![make_room(Some(1), 0.0, 0.0, 4.0, 4.0)],
        16.0,
    );
    (world, room_id)
}

fn create_emitter(
    ecs: &mut Ecs,
    position: Vec2,
    z: i32,
    layer: RoomLayer,
) -> Entity {
    ecs.create_entity()
        .with(ParticleEmitter {
            config: ParticleEmitterConfig {
                burst_count: 2,
                ..ParticleEmitterConfig::default()
            },
        })
        .with(Transform {
            position,
            z,
            ..Transform::default()
        })
        .with_current_room_layer(RoomId(1), layer)
        .finish()
}

#[test]
fn particles_follow_current_owner_layer_and_z_and_sort_after_the_owner_visual() {
    let (world, room_id) = make_room_world();
    let mut ecs = Ecs::default();
    let moved_owner = create_emitter(&mut ecs, Vec2::new(24.0, 24.0), 5, RoomLayer::Front);
    let low_owner = create_emitter(&mut ecs, Vec2::new(40.0, 24.0), 5, RoomLayer::Front);
    let high_owner = create_emitter(&mut ecs, Vec2::new(48.0, 24.0), 5, RoomLayer::Front);
    let low_z_owner = create_emitter(&mut ecs, Vec2::new(8.0, 8.0), 1, RoomLayer::Front);
    let mut particles = ParticleSystem::with_capacity(8);

    assert!(particles.burst(high_owner, &ecs).is_ok());
    assert!(particles.burst(low_owner, &ecs).is_ok());
    assert!(particles.burst(moved_owner, &ecs).is_ok());
    let Some(transform) = ecs.get_mut::<Transform>(moved_owner) else {
        panic!("emitter is missing its Transform component");
    };
    transform.z = -2;
    ecs.set_entity_layer(moved_owner, RoomLayer::Back);

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        Some(&particles),
    );

    let front = layer_maps.for_layer(RoomLayer::Front);
    assert_eq!(front.keys().copied().collect::<Vec<_>>(), vec![1, 5]);
    let Some(same_z_layer) = front.get(&5) else {
        panic!("expected front-layer z=5 draw data");
    };
    let same_z_items = &same_z_layer.items;
    let item_keys = same_z_items
        .iter()
        .map(|item| match item {
            LayerDrawItem::Entity { entity, .. } => (*entity, 0, 0),
            LayerDrawItem::Particle { particle, .. } => {
                (particle.owner, 1, particle.emission_sequence)
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        item_keys,
        vec![
            (low_owner, 0, 0),
            (low_owner, 1, 2),
            (low_owner, 1, 3),
            (high_owner, 0, 0),
            (high_owner, 1, 0),
            (high_owner, 1, 1),
        ]
    );

    let back = layer_maps.for_layer(RoomLayer::Back);
    let Some(moved_layer) = back.get(&-2) else {
        panic!("expected back-layer z=-2 draw data");
    };
    let moved_items = &moved_layer.items;
    assert_eq!(
        moved_items
            .iter()
            .map(|item| match item {
                LayerDrawItem::Entity { entity, .. } => *entity,
                LayerDrawItem::Particle { particle, .. } => particle.owner,
            })
            .collect::<Vec<_>>(),
        vec![moved_owner, moved_owner, moved_owner],
    );
    assert!(!front
        .values()
        .flat_map(|layer| &layer.items)
        .any(|item| matches!(item, LayerDrawItem::Particle { particle, .. } if particle.owner == moved_owner)));
    assert!(ecs.has::<ParticleEmitter>(low_z_owner));
}

#[test]
fn particles_spill_into_a_neighbor_room_only_through_an_exit_cell() {
    let exit = Exit {
        position: Vec2::new(1.0, 4.0),
        direction: ExitDirection::Down,
        layer: RoomLayer::Front,
        target_room_id: Some(RoomId(2)),
    };
    let (world, mut ecs, owner, room_id) =
        make_vertical_spillover_fixture(Vec2::new(24.0, 63.0), Some(exit));
    ecs.add_component_to_entity(
        owner,
        ParticleEmitter {
            config: ParticleEmitterConfig {
                burst_count: 1,
                area_size: Vec2::ZERO,
                ..ParticleEmitterConfig::default()
            },
        },
    );
    let mut particles = ParticleSystem::with_capacity(2);
    assert!(particles.burst(owner, &ecs).is_ok());

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        Some(&particles),
    );
    let visible = layer_maps
        .for_layer(RoomLayer::Front)
        .values()
        .flat_map(|layer| &layer.items)
        .any(|item| matches!(item, LayerDrawItem::Particle { particle, .. } if particle.owner == owner));

    assert!(visible);
}

#[test]
fn particles_from_neighbor_rooms_are_hidden_without_an_opening() {
    let (world, mut ecs, owner, room_id) =
        make_vertical_spillover_fixture(Vec2::new(24.0, 63.0), None);
    ecs.add_component_to_entity(
        owner,
        ParticleEmitter {
            config: ParticleEmitterConfig {
                burst_count: 1,
                area_size: Vec2::ZERO,
                ..ParticleEmitterConfig::default()
            },
        },
    );
    let mut particles = ParticleSystem::with_capacity(2);
    assert!(particles.burst(owner, &ecs).is_ok());

    let layer_maps = collect_interpolated_room_layer_maps(
        &ecs,
        &room_visibility_context(&world, room_id),
        &SpriteManager::default(),
        1.0,
        None,
        Some(&particles),
    );
    let visible = layer_maps
        .for_layer(RoomLayer::Front)
        .values()
        .flat_map(|layer| &layer.items)
        .any(|item| matches!(item, LayerDrawItem::Particle { particle, .. } if particle.owner == owner));

    assert!(!visible);
}
