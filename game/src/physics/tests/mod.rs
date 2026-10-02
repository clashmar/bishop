use crate::physics::physics_system::update_physics as update_physics_system;
use crate::physics::runtime::PhysicsRuntime;
use engine_core::ecs::Ecs;
use engine_core::worlds::World;

pub(crate) fn update_physics(ecs: &mut Ecs, world: &World, dt: f32) {
    let mut runtime = PhysicsRuntime::default();
    update_physics_system(ecs, world, dt, &mut runtime);
}

mod collision_tests;
mod gravity_tests;
mod kinematic_tests;
mod physics_body_tests;
mod physics_fixtures;
mod physics_system_tests;
mod sensor_tests;
mod shapes_tests;
