use bishop::prelude::Vec2;
use engine_core::ecs::{
    Active,
    Collider,
    ColliderShape,
    Ecs,
    Entity,
    Grounded,
    PhysicsBody,
    Pivot,
    Solid,
    SubPixel,
    Transform,
    Velocity,
    true_position,
};

use crate::physics::tests::physics_fixtures::{
    spawn_aabb_physics_body,
    single_room_test_world,
    DT,
    TEST_ROOM_ID,
};
use crate::physics::tests::update_physics;

#[test]
fn moving_solid_physics_body_does_not_overlap_non_solid_physics_body() {
    let mut ecs = Ecs::default();
    let player = spawn_aabb_physics_body(
        &mut ecs,
        TEST_ROOM_ID,
        Vec2::new(36.0, 24.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(120.0, 0.0),
        true,
    );
    let dog = spawn_aabb_physics_body(
        &mut ecs,
        TEST_ROOM_ID,
        Vec2::new(48.0, 24.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(-180.0, 0.0),
        true,
    );
    ecs.add_component_to_entity(dog, Solid(true));
    let world = single_room_test_world();

    update_physics(&mut ecs, &world, DT);

    let Some(player_position) = ecs.get::<Transform>(player).map(|transform| transform.position) else {
        panic!("player is missing Transform");
    };
    let Some(dog_position) = ecs.get::<Transform>(dog).map(|transform| transform.position) else {
        panic!("dog is missing Transform");
    };

    assert!(!ecs.has::<Solid>(player));
    assert!(ecs.get::<Solid>(dog).is_some_and(|solid| solid.0));
    assert!(dog_position.x < 48.0, "solid physics body should move: {dog_position:?}");
    assert!(
        player_position.x + 8.0 <= dog_position.x || dog_position.x + 8.0 <= player_position.x,
        "non-solid physics body overlaps moving solid: player={player_position:?}, dog={dog_position:?}"
    );
}

#[test]
fn moving_solid_physics_body_cannot_enter_non_solid_physics_body() {
    let mut ecs = Ecs::default();
    let player = spawn_aabb_physics_body(
        &mut ecs,
        TEST_ROOM_ID,
        Vec2::new(36.0, 24.0),
        Vec2::new(8.0, 8.0),
        Vec2::ZERO,
        true,
    );
    let dog = spawn_aabb_physics_body(
        &mut ecs,
        TEST_ROOM_ID,
        Vec2::new(48.0, 24.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(-600.0, 0.0),
        true,
    );
    ecs.add_component_to_entity(dog, Solid(true));
    let world = single_room_test_world();

    update_physics(&mut ecs, &world, DT);

    let Some(player_position) = ecs.get::<Transform>(player).map(|transform| transform.position) else {
        panic!("player is missing Transform");
    };
    let Some(dog_position) = ecs.get::<Transform>(dog).map(|transform| transform.position) else {
        panic!("dog is missing Transform");
    };

    assert!(!ecs.has::<Solid>(player));
    assert!(dog_position.x >= player_position.x + 8.0);
}

#[test]
fn moving_solid_and_non_solid_bodies_do_not_overlap_during_concurrent_motion() {
    let velocities = [
        Vec2::new(600.0, 0.0),
        Vec2::new(-600.0, 0.0),
        Vec2::new(0.0, 600.0),
        Vec2::new(0.0, -600.0),
        Vec2::new(600.0, 600.0),
        Vec2::new(-600.0, 600.0),
        Vec2::new(600.0, -600.0),
        Vec2::new(-600.0, -600.0),
    ];

    for player_velocity in velocities {
        for dog_velocity in velocities {
            let mut ecs = Ecs::default();
            let player = spawn_aabb_physics_body(
                &mut ecs,
                TEST_ROOM_ID,
                Vec2::new(36.0, 24.0),
                Vec2::new(8.0, 8.0),
                Vec2::ZERO,
                true,
            );
            let dog = spawn_aabb_physics_body(
                &mut ecs,
                TEST_ROOM_ID,
                Vec2::new(48.0, 24.0),
                Vec2::new(8.0, 8.0),
                Vec2::ZERO,
                true,
            );
            ecs.add_component_to_entity(dog, Solid(true));
            let world = single_room_test_world();

            for _ in 0..60 {
                if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
                    velocity.x = player_velocity.x;
                    velocity.y = player_velocity.y;
                }
                if let Some(velocity) = ecs.get_mut::<Velocity>(dog) {
                    velocity.x = dog_velocity.x;
                    velocity.y = dog_velocity.y;
                }

                update_physics(&mut ecs, &world, DT);

                let player_position = ecs.get::<Transform>(player).unwrap().position;
                let dog_position = ecs.get::<Transform>(dog).unwrap().position;
                let overlap_x = player_position.x < dog_position.x + 8.0
                    && player_position.x + 8.0 > dog_position.x;
                let overlap_y = player_position.y < dog_position.y + 8.0
                    && player_position.y + 8.0 > dog_position.y;
                assert!(
                    !overlap_x || !overlap_y,
                    "actors overlapped: player_velocity={player_velocity:?}, dog_velocity={dog_velocity:?}, player={player_position:?}, dog={dog_position:?}"
                );
            }
        }
    }
}

#[test]
fn bottom_center_demo_sized_actors_do_not_overlap_when_both_move() {
    let mut ecs = Ecs::default();
    let player = spawn_bottom_center_body(
        &mut ecs,
        Vec2::new(100.0, 72.0),
        Vec2::new(6.0, 16.0),
        Vec2::new(180.0, 0.0),
    );
    let dog = spawn_bottom_center_body(
        &mut ecs,
        Vec2::new(112.0, 72.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(-48.0, 0.0),
    );
    ecs.add_component_to_entity(dog, Solid(true));
    let world = single_room_test_world();

    for _ in 0..30 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = 180.0;
        }
        if let Some(velocity) = ecs.get_mut::<Velocity>(dog) {
            velocity.x = -48.0;
        }
        update_physics(&mut ecs, &world, DT);

        let player_position = ecs.get::<Transform>(player).unwrap().position;
        let dog_position = ecs.get::<Transform>(dog).unwrap().position;
        assert!(
            player_position.x + 3.0 <= dog_position.x - 4.0,
            "demo-sized actors overlapped: player={player_position:?}, dog={dog_position:?}"
        );
    }
}

#[test]
fn jumping_player_and_moving_solid_do_not_overlap() {
    let mut ecs = Ecs::default();
    let player = spawn_bottom_center_body(
        &mut ecs,
        Vec2::new(100.0, 72.0),
        Vec2::new(6.0, 16.0),
        Vec2::new(180.0, -200.0),
    );
    let dog = spawn_bottom_center_body(
        &mut ecs,
        Vec2::new(112.0, 72.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(-48.0, 0.0),
    );
    ecs.add_component_to_entity(dog, Solid(true));
    let world = single_room_test_world();

    for _ in 0..60 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = 180.0;
            velocity.y = -200.0;
        }
        if let Some(velocity) = ecs.get_mut::<Velocity>(dog) {
            velocity.x = -48.0;
        }
        update_physics(&mut ecs, &world, DT);

        let player_position = ecs.get::<Transform>(player).unwrap().position;
        let dog_position = ecs.get::<Transform>(dog).unwrap().position;
        let overlap_x = player_position.x - 3.0 < dog_position.x + 4.0
            && player_position.x + 3.0 > dog_position.x - 4.0;
        let overlap_y = player_position.y - 16.0 < dog_position.y
            && player_position.y > dog_position.y - 8.0;
        assert!(
            !overlap_x || !overlap_y,
            "jumping player and Dog overlapped: player={player_position:?}, dog={dog_position:?}"
        );
    }
}

#[test]
fn blocked_subpixel_contact_does_not_snap_solid_actor_into_player() {
    let mut ecs = Ecs::default();
    let player = spawn_aabb_physics_body(
        &mut ecs,
        TEST_ROOM_ID,
        Vec2::new(44.0, 78.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(562.0, -181.0),
        true,
    );
    let dog = spawn_aabb_physics_body(
        &mut ecs,
        TEST_ROOM_ID,
        Vec2::new(53.0, 70.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(407.0, 532.0),
        true,
    );
    ecs.get_mut::<SubPixel>(player).unwrap().x = -0.1000061;
    ecs.get_mut::<SubPixel>(player).unwrap().y = -0.3666687;
    ecs.get_mut::<SubPixel>(dog).unwrap().x = 0.166664;
    ecs.get_mut::<SubPixel>(dog).unwrap().y = 0.0833359;
    ecs.add_component_to_entity(dog, Solid(true));
    let world = single_room_test_world();

    update_physics(&mut ecs, &world, DT);

    let player_position = true_position(
        ecs.get::<Transform>(player).unwrap().position,
        ecs.get::<SubPixel>(player).copied().unwrap_or_default(),
    );
    let dog_position = true_position(
        ecs.get::<Transform>(dog).unwrap().position,
        ecs.get::<SubPixel>(dog).copied().unwrap_or_default(),
    );
    assert!(
        player_position.x + 8.0 <= dog_position.x
            || dog_position.x + 8.0 <= player_position.x
            || player_position.y + 8.0 <= dog_position.y
            || dog_position.y + 8.0 <= player_position.y,
        "solid and non-solid actors overlapped: player={player_position:?}, dog={dog_position:?}"
    );
}

#[test]
fn solid_actor_never_overlaps_non_solid_actor_across_varied_motion() {
    let mut seed = 0x5eed_u32;
    for scenario in 0..100 {
        let player_start = Vec2::new(
            24.0 + (next_motion_sample(&mut seed) % 80) as f32,
            24.0 + (next_motion_sample(&mut seed) % 80) as f32,
        );
        let mut dog_start = Vec2::new(
            24.0 + (next_motion_sample(&mut seed) % 80) as f32,
            24.0 + (next_motion_sample(&mut seed) % 80) as f32,
        );
        if player_start.x < dog_start.x + 8.0
            && player_start.x + 8.0 > dog_start.x
            && player_start.y < dog_start.y + 8.0
            && player_start.y + 8.0 > dog_start.y
        {
            dog_start.x += 12.0;
        }

        let mut ecs = Ecs::default();
        let player = spawn_aabb_physics_body(
            &mut ecs,
            TEST_ROOM_ID,
            player_start,
            Vec2::new(8.0, 8.0),
            Vec2::ZERO,
            true,
        );
        let dog = spawn_aabb_physics_body(
            &mut ecs,
            TEST_ROOM_ID,
            dog_start,
            Vec2::new(8.0, 8.0),
            Vec2::ZERO,
            true,
        );
        ecs.add_component_to_entity(dog, Solid(true));
        let world = single_room_test_world();

        for frame in 0..30 {
            let player_velocity = Vec2::new(
                (next_motion_sample(&mut seed) % 1201) as f32 - 600.0,
                (next_motion_sample(&mut seed) % 1201) as f32 - 600.0,
            );
            let dog_velocity = Vec2::new(
                (next_motion_sample(&mut seed) % 1201) as f32 - 600.0,
                (next_motion_sample(&mut seed) % 1201) as f32 - 600.0,
            );
            if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
                velocity.x = player_velocity.x;
                velocity.y = player_velocity.y;
            }
            if let Some(velocity) = ecs.get_mut::<Velocity>(dog) {
                velocity.x = dog_velocity.x;
                velocity.y = dog_velocity.y;
            }
            let player_before = true_position(
                ecs.get::<Transform>(player).unwrap().position,
                ecs.get::<SubPixel>(player).copied().unwrap_or_default(),
            );
            let dog_before = true_position(
                ecs.get::<Transform>(dog).unwrap().position,
                ecs.get::<SubPixel>(dog).copied().unwrap_or_default(),
            );

            update_physics(&mut ecs, &world, DT);

            let player_transform = ecs.get::<Transform>(player).unwrap();
            let dog_transform = ecs.get::<Transform>(dog).unwrap();
            let player_position = true_position(
                player_transform.position,
                ecs.get::<SubPixel>(player).copied().unwrap_or_default(),
            );
            let dog_position = true_position(
                dog_transform.position,
                ecs.get::<SubPixel>(dog).copied().unwrap_or_default(),
            );
            let overlap_x = player_position.x < dog_position.x + 8.0 - 0.0001
                && player_position.x + 8.0 > dog_position.x + 0.0001;
            let overlap_y = player_position.y < dog_position.y + 8.0 - 0.0001
                && player_position.y + 8.0 > dog_position.y + 0.0001;
            assert!(
                !overlap_x || !overlap_y,
                "scenario {scenario}, frame {frame}, start_player={player_start:?}, start_dog={dog_start:?}, before_player={player_before:?}, before_dog={dog_before:?}, player_velocity={player_velocity:?}, dog_velocity={dog_velocity:?}, player={player_position:?}, dog={dog_position:?}"
            );
        }
    }
}

#[test]
fn moving_non_solid_physics_bodies_can_overlap() {
    let mut ecs = Ecs::default();
    let player = spawn_aabb_physics_body(
        &mut ecs,
        TEST_ROOM_ID,
        Vec2::new(36.0, 24.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(120.0, 0.0),
        true,
    );
    let other = spawn_aabb_physics_body(
        &mut ecs,
        TEST_ROOM_ID,
        Vec2::new(48.0, 24.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(-180.0, 0.0),
        true,
    );
    let world = single_room_test_world();

    update_physics(&mut ecs, &world, DT);

    let Some(player_position) = ecs.get::<Transform>(player).map(|transform| transform.position) else {
        panic!("player is missing Transform");
    };
    let Some(other_position) = ecs.get::<Transform>(other).map(|transform| transform.position) else {
        panic!("other body is missing Transform");
    };

    assert!(player_position.x + 8.0 > other_position.x);
}

fn spawn_bottom_center_body(
    ecs: &mut Ecs,
    position: Vec2,
    size: Vec2,
    velocity: Vec2,
) -> Entity {
    ecs.create_entity()
        .with_current_room(TEST_ROOM_ID)
        .with(Transform {
            position,
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: size.x,
                height: size.y,
            },
            ..Default::default()
        })
        .with(Velocity {
            x: velocity.x,
            y: velocity.y,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with(Active::default())
        .finish()
}

fn next_motion_sample(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed
}
