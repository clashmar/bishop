use bishop::prelude::*;
use crate::constants::GRAVITY;
use crate::physics::collision::SweepContext;
use engine_core::assets::*;
use engine_core::ecs::*;
use engine_core::worlds::*;

/// Applies fixed-step movement to `MotionBody`s and full collision physics to `PhysicsBody`s.
pub fn update_physics(
    sprite_manager: &SpriteManager,
    ecs: &mut Ecs,
    world: &World,
    dt: f32,
) {
    update_motion_bodies(ecs, world, dt);

    let entities: Vec<_> = ecs
        .get_store::<PhysicsBody>()
        .data
        .keys()
        .filter(|entity| ecs.get::<Active>(**entity).is_some_and(|active| active.0))
        .copied()
        .collect();

    for entity in entities {
        let (pos_cur, pivot, mut vel_cur, collider) = {
            let Some(transform) = ecs.get::<Transform>(entity).copied() else {
                continue;
            };
            let Some(velocity) = ecs.get::<Velocity>(entity).copied() else {
                continue;
            };
            let collider = ecs.get::<Collider>(entity).copied().unwrap_or_default();
            (transform.position, transform.pivot, velocity, collider)
        };

        let Some(room_id) = ecs.get::<CurrentRoom>(entity).map(|room| room.0) else {
            continue;
        };
        let Some(room) = world.get_room(room_id) else {
            continue;
        };
        let tilemap = &room.variants[room.current_variant_index()].tilemap;

        let mut sub_pixel = ecs.get::<SubPixel>(entity).copied().unwrap_or_default();

        vel_cur.y += GRAVITY * dt;

        let delta = Vec2::new(vel_cur.x * dt, vel_cur.y * dt);

        // Sweep from the true float position (integer + sub-pixel remainder)
        // so collision detection measures distances correctly.
        let true_pos = pos_cur + Vec2::new(sub_pixel.x, sub_pixel.y);

        let collision_world = SweepContext::new(
            sprite_manager,
            ecs,
            room_id,
            tilemap,
            room.position,
            &room.exits,
            world.grid_size,
        );
        let sweep = collision_world.sweep_move(entity, true_pos, delta, collider, pivot);

        // Snap to integer positions, storing the fractional part for next frame
        let (new_int_pos, new_sub_pixel) = quantize_motion(pos_cur, sub_pixel, sweep.allowed_delta);
        sub_pixel = new_sub_pixel;

        let was_falling = vel_cur.y >= 0.0;

        // On collision, zero out velocity and discard sub-pixel remainder
        if sweep.blocked_x {
            vel_cur.x = 0.0;
            sub_pixel.x = 0.0;
        }
        if sweep.blocked_y {
            vel_cur.y = 0.0;
            sub_pixel.y = 0.0;
        }

        update_entity_position(ecs, entity, new_int_pos);
        if let Some(velocity) = ecs.get_mut::<Velocity>(entity) {
            *velocity = vel_cur;
        }

        if let Some(sp) = ecs.get_mut::<SubPixel>(entity) {
            *sp = sub_pixel;
        }
        if let Some(grounded) = ecs.get_mut::<Grounded>(entity) {
            grounded.0 = sweep.blocked_y && was_falling;
        }
    }
}

fn update_motion_bodies(ecs: &mut Ecs, world: &World, dt: f32) {
    let entities: Vec<_> = ecs
        .get_store::<MotionBody>()
        .data
        .keys()
        .filter(|entity| !ecs.has::<PhysicsBody>(**entity))
        .filter(|entity| entity_in_world(ecs, world, **entity))
        .copied()
        .collect();

    for entity in entities {
        let Some(transform) = ecs.get::<Transform>(entity).copied() else {
            continue;
        };
        let Some(velocity) = ecs.get::<Velocity>(entity).copied() else {
            continue;
        };

        let sub_pixel = ecs.get::<SubPixel>(entity).copied().unwrap_or_default();
        let delta = Vec2::new(velocity.x * dt, velocity.y * dt);
        let (new_int_pos, new_sub_pixel) = quantize_motion(transform.position, sub_pixel, delta);

        update_entity_position(ecs, entity, new_int_pos);
        store_sub_pixel(ecs, entity, new_sub_pixel);
    }
}

fn quantize_motion(position: Vec2, sub_pixel: SubPixel, delta: Vec2) -> (Vec2, SubPixel) {
    let true_pos = position + Vec2::new(sub_pixel.x, sub_pixel.y);
    let new_true_pos = true_pos + delta;
    let new_int_pos = new_true_pos.round();

    (
        new_int_pos,
        SubPixel {
            x: new_true_pos.x - new_int_pos.x,
            y: new_true_pos.y - new_int_pos.y,
        },
    )
}

fn store_sub_pixel(ecs: &mut Ecs, entity: Entity, sub_pixel: SubPixel) {
    if let Some(existing) = ecs.get_mut::<SubPixel>(entity) {
        *existing = sub_pixel;
        return;
    }

    if sub_pixel.x != 0.0 || sub_pixel.y != 0.0 {
        ecs.add_component_to_entity(entity, sub_pixel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::tiles::TileMap;

    fn empty_room() -> Room {
        Room {
            id: RoomId(1),
            variants: vec![RoomVariant::default()],
            ..Default::default()
        }
    }

    fn empty_world() -> World {
        let room = empty_room();
        let mut world = World::default();
        world.current_room_id = Some(room.id);
        world.add_room(room);
        world
    }

    #[test]
    fn motion_bodies_in_inactive_worlds_do_not_move() {
        let mut ecs = Ecs::default();
        let parked = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(10.0, 12.0),
                ..Default::default()
            })
            .with(Velocity { x: 120.0, y: 0.0 })
            .with(MotionBody)
            .with(SubPixel::default())
            .with_current_room(RoomId(99))
            .finish();

        update_physics(&SpriteManager::default(), &mut ecs, &empty_world(), 1.0 / 60.0);

        assert_eq!(
            ecs.get::<Transform>(parked)
                .map(|transform| transform.position),
            Some(Vec2::new(10.0, 12.0))
        );
    }

    #[test]
    fn motion_bodies_roomed_in_the_active_world_still_move() {
        let mut ecs = Ecs::default();
        let roomed = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(10.0, 12.0),
                ..Default::default()
            })
            .with(Velocity { x: 120.0, y: 0.0 })
            .with(MotionBody)
            .with(SubPixel::default())
            .with_current_room(RoomId(1))
            .finish();

        update_physics(&SpriteManager::default(), &mut ecs, &empty_world(), 1.0 / 60.0);

        assert_eq!(
            ecs.get::<Transform>(roomed)
                .map(|transform| transform.position),
            Some(Vec2::new(12.0, 12.0))
        );
    }

    #[test]
    fn velocity_entities_move_with_fixed_step_subpixel_accumulation() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(10.0, 12.0),
                ..Default::default()
            })
            .with(Velocity { x: 120.0, y: 0.0 })
            .with(SubPixel::default())
            .finish();

        update_physics(&SpriteManager::default(), &mut ecs, &empty_world(), 1.0 / 60.0);

        assert_eq!(
            ecs.get::<Transform>(entity)
                .map(|transform| transform.position),
            Some(Vec2::new(12.0, 12.0))
        );
        assert_eq!(
            ecs.get::<SubPixel>(entity)
                .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
            Some((0.0, 0.0))
        );
    }

    #[test]
    fn motion_body_entities_move_with_fixed_step_subpixel_accumulation() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(10.0, 12.0),
                ..Default::default()
            })
            .with(Velocity { x: 120.0, y: 0.0 })
            .with(MotionBody)
            .with(SubPixel::default())
            .finish();

        update_physics(&SpriteManager::default(), &mut ecs, &empty_world(), 1.0 / 60.0);

        assert_eq!(
            ecs.get::<Transform>(entity)
                .map(|transform| transform.position),
            Some(Vec2::new(12.0, 12.0))
        );
        assert_eq!(
            ecs.get::<SubPixel>(entity)
                .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
            Some((0.0, 0.0))
        );
    }

    #[test]
    fn physics_body_entities_accumulate_fractional_motion_in_subpixel() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform::default())
            .with(Velocity { x: 30.0, y: 0.0 })
            .with(PhysicsBody)
            .with_current_room(RoomId(1))
            .with(Active::default())
            .with(SubPixel::default())
            .finish();

        update_physics(&SpriteManager::default(), &mut ecs, &empty_world(), 1.0 / 60.0);

        assert_eq!(
            ecs.get::<Transform>(entity)
                .map(|transform| transform.position),
            Some(Vec2::new(1.0, 0.0))
        );
        assert_eq!(
            ecs.get::<SubPixel>(entity)
                .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
            Some((-0.5, 0.0))
        );
    }

    #[test]
    fn inactive_physics_body_is_not_simulated() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform::default())
            .with(Velocity { x: 30.0, y: 0.0 })
            .with(Collider::default())
            .with(PhysicsBody)
            .with_current_room(RoomId(1))
            .with(Active(false))
            .finish();

        let world = empty_world();

        update_physics(&SpriteManager::default(), &mut ecs, &world, 1.0 / 60.0);

        assert_eq!(
            ecs.get::<Transform>(entity).map(|transform| transform.position),
            Some(Vec2::ZERO)
        );
    }

    #[test]
    fn physics_body_without_current_room_is_not_simulated() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform::default())
            .with(Velocity { x: 30.0, y: 0.0 })
            .with(Collider::default())
            .with(PhysicsBody)
            .with(Active::default())
            .finish();

        let world = empty_world();

        update_physics(&SpriteManager::default(), &mut ecs, &world, 1.0 / 60.0);

        assert_eq!(
            ecs.get::<Transform>(entity).map(|transform| transform.position),
            Some(Vec2::ZERO)
        );
    }

    #[test]
    fn physics_body_uses_its_current_room_instead_of_world_current_room() {
        let room_a = Room {
            id: RoomId(1),
            position: Vec2::ZERO,
            size: Vec2::new(32.0, 32.0),
            variants: vec![RoomVariant {
                tilemap: TileMap::new(2, 2),
                ..Default::default()
            }],
            ..Default::default()
        };
        let room_b = Room {
            id: RoomId(2),
            position: Vec2::new(128.0, 0.0),
            size: Vec2::new(32.0, 32.0),
            variants: vec![RoomVariant {
                tilemap: TileMap::new(2, 2),
                ..Default::default()
            }],
            ..Default::default()
        };

        let mut world = World::default();
        world.grid_size = 16.0;
        world.current_room_id = Some(room_a.id);
        world.add_room(room_a);
        world.add_room(room_b);

        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(152.0, 16.0),
                pivot: Pivot::TopLeft,
                ..Default::default()
            })
            .with_current_room(RoomId(2))
            .with(Active::default())
            .with(PhysicsBody)
            .with(Collider {
                width: 8.0,
                height: 8.0,
            })
            .with(Velocity { x: 60.0, y: 0.0 })
            .finish();

        update_physics(&SpriteManager::default(), &mut ecs, &world, 1.0 / 60.0);

        assert_eq!(
            ecs.get::<Transform>(entity).map(|transform| transform.position.x),
            Some(152.0)
        );
    }

    #[test]
    fn active_physics_body_is_simulated() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform::default())
            .with(Velocity { x: 30.0, y: 0.0 })
            .with(Collider::default())
            .with(PhysicsBody)
            .with_current_room(RoomId(1))
            .with(Active::default())
            .finish();

        let world = empty_world();

        update_physics(&SpriteManager::default(), &mut ecs, &world, 1.0 / 60.0);

        assert_eq!(
            ecs.get::<Transform>(entity).map(|transform| transform.position),
            Some(Vec2::new(1.0, 0.0))
        );
    }
}
