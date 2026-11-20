use super::*;
use crate::ecs::components::{
    Collider, ParticleEmitter, ParticleEmissionMode, ParticleEmitterConfig, PhysicsBody, Transform,
};
use crate::ecs::{Ecs, Entity};
use crate::worlds::room::RoomId;
use bishop::types::Vec2;

fn create_emitter(ecs: &mut Ecs, config: ParticleEmitterConfig, position: Vec2) -> Entity {
    ecs.create_entity()
        .with(ParticleEmitter { config })
        .with(Transform {
            position,
            ..Transform::default()
        })
        .with_current_room(RoomId(1))
        .finish()
}

#[test]
fn burst_is_reproducible_for_the_same_seed() {
    let config = ParticleEmitterConfig {
        burst_count: 16,
        area_size: Vec2::new(12.0, 8.0),
        ..ParticleEmitterConfig::default()
    };
    let mut ecs_a = Ecs::default();
    let mut ecs_b = Ecs::default();
    let owner_a = create_emitter(&mut ecs_a, config.clone(), Vec2::new(50.0, 70.0));
    let owner_b = create_emitter(&mut ecs_b, config, Vec2::new(50.0, 70.0));
    let mut system_a = ParticleSystem::with_capacity(32);
    let mut system_b = ParticleSystem::with_capacity(32);

    system_a.burst(owner_a, &ecs_a).unwrap();
    system_b.burst(owner_b, &ecs_b).unwrap();

    assert_eq!(
        system_a.particles().cloned().collect::<Vec<_>>(),
        system_b.particles().cloned().collect::<Vec<_>>()
    );
}

#[test]
fn burst_samples_positions_inside_the_emission_area() {
    let config = ParticleEmitterConfig {
        burst_count: 64,
        area_size: Vec2::new(10.0, 4.0),
        velocity_min: Vec2::ZERO,
        velocity_max: Vec2::ZERO,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    let owner = create_emitter(&mut ecs, config, Vec2::new(100.0, 200.0));
    let mut system = ParticleSystem::with_capacity(64);

    system.burst(owner, &ecs).unwrap();

    for particle in system.particles() {
        assert!((95.0..=105.0).contains(&particle.position.x));
        assert!((198.0..=202.0).contains(&particle.position.y));
    }
}

#[test]
fn continuous_emission_accumulates_fractional_rates() {
    let config = ParticleEmitterConfig {
        mode: ParticleEmissionMode::Continuous,
        emission_rate: 2.0,
        lifetime_min: 10.0,
        lifetime_max: 10.0,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    create_emitter(&mut ecs, config, Vec2::ZERO);
    let mut system = ParticleSystem::with_capacity(8);

    system.update(0.25, &ecs);
    assert_eq!(system.len(), 0);

    system.update(0.25, &ecs);
    assert_eq!(system.len(), 1);
}

#[test]
fn particles_expire_and_interpolate_scale_and_opacity() {
    let config = ParticleEmitterConfig {
        burst_count: 1,
        lifetime_min: 1.0,
        lifetime_max: 1.0,
        velocity_min: Vec2::ZERO,
        velocity_max: Vec2::ZERO,
        scale_start: 2.0,
        scale_end: 0.0,
        opacity_start: 1.0,
        opacity_end: 0.0,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    let owner = create_emitter(&mut ecs, config, Vec2::ZERO);
    let mut system = ParticleSystem::with_capacity(2);

    system.burst(owner, &ecs).unwrap();
    system.update(0.5, &ecs);

    let particle = system.particles().next().unwrap();
    assert!((particle.scale - 1.0).abs() < 0.0001);
    assert!((particle.opacity - 0.5).abs() < 0.0001);

    system.update(0.5, &ecs);
    assert_eq!(system.len(), 0);
}

#[test]
fn per_emitter_limit_drops_excess_particles() {
    let config = ParticleEmitterConfig {
        burst_count: 5,
        max_particles: 2,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    let owner = create_emitter(&mut ecs, config, Vec2::ZERO);
    let mut system = ParticleSystem::with_capacity(8);

    system.burst(owner, &ecs).unwrap();
    system.burst(owner, &ecs).unwrap();

    assert_eq!(system.len(), 2);
}

#[test]
fn global_pool_capacity_drops_excess_particles() {
    let config = ParticleEmitterConfig {
        burst_count: 4,
        max_particles: 8,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    let owner_a = create_emitter(&mut ecs, config.clone(), Vec2::ZERO);
    let owner_b = create_emitter(&mut ecs, config, Vec2::ZERO);
    let mut system = ParticleSystem::with_capacity(3);

    system.burst(owner_a, &ecs).unwrap();
    system.burst(owner_b, &ecs).unwrap();

    assert_eq!(system.len(), 3);
}

#[test]
fn expired_slots_are_reused() {
    let config = ParticleEmitterConfig {
        burst_count: 1,
        lifetime_min: 0.5,
        lifetime_max: 0.5,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    let owner = create_emitter(&mut ecs, config, Vec2::ZERO);
    let mut system = ParticleSystem::with_capacity(1);

    system.burst(owner, &ecs).unwrap();
    system.update(0.5, &ecs);
    assert_eq!(system.len(), 0);

    system.burst(owner, &ecs).unwrap();
    assert_eq!(system.len(), 1);
}

#[test]
fn changing_rooms_releases_owned_particles() {
    let config = ParticleEmitterConfig {
        burst_count: 2,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    let owner = create_emitter(&mut ecs, config, Vec2::ZERO);
    let mut system = ParticleSystem::with_capacity(4);
    system.burst(owner, &ecs).unwrap();
    assert_eq!(system.len(), 2);

    ecs.set_current_room(owner, RoomId(2));
    system.update(0.01, &ecs);

    assert_eq!(system.len(), 0);
}

#[test]
fn removing_emitter_component_releases_owned_particles() {
    let config = ParticleEmitterConfig {
        burst_count: 2,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    let owner = create_emitter(&mut ecs, config, Vec2::ZERO);
    let mut system = ParticleSystem::with_capacity(4);
    system.burst(owner, &ecs).unwrap();
    assert_eq!(system.len(), 2);

    ecs.get_store_mut::<ParticleEmitter>().remove(owner);
    system.update(0.01, &ecs);

    assert_eq!(system.len(), 0);
}

#[test]
fn emission_does_not_create_entities_or_mutate_physics_components() {
    let config = ParticleEmitterConfig {
        burst_count: 2,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    let owner = ecs
        .create_entity()
        .with(ParticleEmitter { config })
        .with(Transform::default())
        .with(Collider::default())
        .with(PhysicsBody)
        .with_current_room(RoomId(1))
        .finish();
    let next_entity_id = ecs.next_entity_id;
    let collider = ecs.get::<Collider>(owner).unwrap().clone();
    assert!(ecs.has::<PhysicsBody>(owner));
    let mut system = ParticleSystem::with_capacity(4);

    system.burst(owner, &ecs).unwrap();
    system.update(0.1, &ecs);

    assert_eq!(ecs.next_entity_id, next_entity_id);
    assert_eq!(ecs.get::<Collider>(owner), Some(&collider));
    assert!(ecs.has::<PhysicsBody>(owner));
}

#[test]
fn invalid_configuration_is_rejected_without_replacing_settings() {
    let mut ecs = Ecs::default();
    let owner = create_emitter(
        &mut ecs,
        ParticleEmitterConfig::default(),
        Vec2::ZERO,
    );
    let mut system = ParticleSystem::with_capacity(4);
    let invalid_config = ParticleEmitterConfig {
        lifetime_min: 0.0,
        ..ParticleEmitterConfig::default()
    };

    assert!(system.configure(owner, invalid_config, &mut ecs).is_err());
    assert_eq!(
        ecs.get::<ParticleEmitter>(owner).unwrap().config,
        ParticleEmitterConfig::default()
    );
}

#[test]
fn start_and_stop_control_continuous_emission_without_catch_up() {
    let config = ParticleEmitterConfig {
        mode: ParticleEmissionMode::Continuous,
        emission_rate: 2.0,
        lifetime_min: 10.0,
        lifetime_max: 10.0,
        ..ParticleEmitterConfig::default()
    };
    let mut ecs = Ecs::default();
    let owner = create_emitter(&mut ecs, config, Vec2::ZERO);
    let mut system = ParticleSystem::with_capacity(8);

    system.update(0.5, &ecs);
    assert_eq!(system.len(), 1);
    system.stop(owner, &ecs).unwrap();
    system.update(1.0, &ecs);
    assert_eq!(system.len(), 1);

    system.start(owner, &ecs).unwrap();
    system.update(0.25, &ecs);
    assert_eq!(system.len(), 1);
    system.update(0.25, &ecs);
    assert_eq!(system.len(), 2);
}
