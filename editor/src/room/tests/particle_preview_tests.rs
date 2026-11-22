use super::ParticlePreview;
use bishop::prelude::Vec2;
use engine_core::ecs::components::{
    CurrentRoom, ParticleEmissionMode, ParticleEmitter, ParticleEmitterConfig, Transform,
};
use engine_core::ecs::{Ecs, Entity};
use engine_core::worlds::{RoomId, RoomLayer};

fn emitter_entity(mode: ParticleEmissionMode) -> (Ecs, Entity) {
    let mut ecs = Ecs::default();
    let config = ParticleEmitterConfig {
        mode,
        area_size: Vec2::ZERO,
        lifetime_min: 1.0,
        lifetime_max: 1.0,
        velocity_min: Vec2::ZERO,
        velocity_max: Vec2::ZERO,
        burst_count: 1,
        emission_rate: 10.0,
        ..ParticleEmitterConfig::default()
    };
    let owner = ecs
        .create_entity()
        .with(Transform::default())
        .with(ParticleEmitter { config })
        .finish();
    ecs.add_component_to_entity(owner, CurrentRoom::new(RoomId(1), RoomLayer::Front));
    (ecs, owner)
}

#[test]
fn burst_preview_advances_and_expires_its_particles() {
    let (ecs, owner) = emitter_entity(ParticleEmissionMode::Burst);
    let mut preview = ParticlePreview::default();

    preview.burst(owner, &ecs).expect("valid burst should start preview");
    assert!(preview.is_active());
    assert_eq!(preview.live_particle_count(), 1);

    preview.update(1.1, Some(owner), &ecs);

    assert!(!preview.is_active());
    assert_eq!(preview.live_particle_count(), 0);
}

#[test]
fn repeated_burst_previews_vary_but_use_the_configured_seed_as_input() {
    let (mut ecs, owner) = emitter_entity(ParticleEmissionMode::Burst);
    let config = &mut ecs
        .get_mut::<ParticleEmitter>(owner)
        .expect("burst emitter should exist")
        .config;
    config.area_size = Vec2::new(100.0, 100.0);
    config.velocity_min = Vec2::new(-50.0, -50.0);
    config.velocity_max = Vec2::new(50.0, 50.0);
    config.lifetime_min = 0.1;
    config.lifetime_max = 1.0;
    config.burst_count = 4;
    let configured_seed = config.seed;
    let mut preview = ParticlePreview::default();

    preview.burst(owner, &ecs).expect("valid burst should preview");
    let first_burst = preview
        .render_system()
        .expect("burst should render")
        .particles()
        .map(|particle| (particle.position, particle.velocity, particle.lifetime))
        .collect::<Vec<_>>();
    preview.burst(owner, &ecs).expect("second burst should preview");
    let all_bursts = preview
        .render_system()
        .expect("second burst should render")
        .particles()
        .map(|particle| (particle.position, particle.velocity, particle.lifetime))
        .collect::<Vec<_>>();

    let mut replay = ParticlePreview::default();
    replay.burst(owner, &ecs).expect("first replay burst should preview");
    let replay_first = replay
        .render_system()
        .expect("first replay burst should render")
        .particles()
        .map(|particle| (particle.position, particle.velocity, particle.lifetime))
        .collect::<Vec<_>>();
    replay.burst(owner, &ecs).expect("second replay burst should preview");
    let replay_all = replay
        .render_system()
        .expect("second replay burst should render")
        .particles()
        .map(|particle| (particle.position, particle.velocity, particle.lifetime))
        .collect::<Vec<_>>();

    assert_eq!(configured_seed, ecs.get::<ParticleEmitter>(owner).unwrap().config.seed);
    assert_ne!(first_burst, all_bursts[4..]);
    assert_eq!(first_burst, replay_first);
    assert_eq!(all_bursts[4..], replay_all[4..]);
}

#[test]
fn stopping_continuous_preview_preserves_live_particles_until_expiry() {
    let (ecs, owner) = emitter_entity(ParticleEmissionMode::Continuous);
    let mut preview = ParticlePreview::default();

    preview.start(owner, &ecs).expect("valid continuous emitter should start");
    preview.update(0.1, Some(owner), &ecs);
    assert_eq!(preview.live_particle_count(), 1);

    preview.stop(owner, &ecs).expect("valid continuous emitter should stop");
    preview.update(0.1, Some(owner), &ecs);
    assert_eq!(preview.live_particle_count(), 1);

    preview.update(1.1, Some(owner), &ecs);
    assert_eq!(preview.live_particle_count(), 0);
    assert!(!preview.is_active());
}

#[test]
fn changing_emitter_mode_stops_continuous_preview_emission() {
    let (mut ecs, owner) = emitter_entity(ParticleEmissionMode::Continuous);
    let mut preview = ParticlePreview::default();

    preview.start(owner, &ecs).expect("valid continuous emitter should start");
    preview.update(0.1, Some(owner), &ecs);
    assert!(preview.is_emitting(owner));

    ecs.get_mut::<ParticleEmitter>(owner)
        .expect("emitter should remain attached")
        .config
        .mode = ParticleEmissionMode::Burst;
    preview.update(0.1, Some(owner), &ecs);

    assert!(!preview.is_emitting(owner));
    assert!(preview.live_particle_count() > 0);
}
