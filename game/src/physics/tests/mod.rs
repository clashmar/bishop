use crate::physics::events::PhysicsEvents;
use crate::physics::physics_system::update_physics_with_events;
use engine_core::ecs::Ecs;
use engine_core::worlds::World;

pub(crate) fn update_physics(ecs: &mut Ecs, world: &World, dt: f32) {
    let mut events = PhysicsEvents::default();
    update_physics_with_events(ecs, world, dt, &mut events);
}

mod gravity_tests;
mod kinematic_tests;
mod physics_body_tests;
mod physics_system_tests;
mod shapes_tests;
