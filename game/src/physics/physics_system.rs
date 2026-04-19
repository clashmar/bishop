// game/src/physics/physics_system.rs
use crate::constants::GRAVITY;
use crate::physics::collision::SweepContext;
use engine_core::prelude::*;

/// Applies fixed-step movement to `MotionBody`s and full collision physics to `PhysicsBody`s.
pub fn update_physics(
    sprite_manager: &SpriteManager,
    ecs: &mut Ecs,
    room: &Room,
    dt: f32,
    grid_size: f32,
) {
    let tilemap = &room.variants[room.current_variant_index()].tilemap;

    update_motion_bodies(ecs, dt);

    let entities: Vec<_> = ecs
        .get_store::<PhysicsBody>()
        .data
        .keys()
        .cloned()
        .collect();

    for entity in entities {
        let (pos_cur, pivot, mut vel_cur, collider) = {
            let t = ecs.get::<Transform>(entity).unwrap();
            let v = ecs.get::<Velocity>(entity).unwrap();
            let c = ecs.get::<Collider>(entity).cloned().unwrap_or_default();
            (t.position, t.pivot, *v, c)
        };

        let mut sub_pixel = ecs.get::<SubPixel>(entity).copied().unwrap_or_default();

        vel_cur.y += GRAVITY * dt;

        let delta = Vec2::new(vel_cur.x * dt, vel_cur.y * dt);

        // Sweep from the true float position (integer + sub-pixel remainder)
        // so collision detection measures distances correctly.
        let true_pos = pos_cur + Vec2::new(sub_pixel.x, sub_pixel.y);

        let collision_world = SweepContext::new(
            sprite_manager,
            ecs,
            tilemap,
            room.position,
            &room.exits,
            grid_size,
        );
        let sweep = collision_world.sweep_move(true_pos, delta, collider, pivot);

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
        *ecs.get_mut::<Velocity>(entity).unwrap() = vel_cur;

        if let Some(sp) = ecs.get_mut::<SubPixel>(entity) {
            *sp = sub_pixel;
        }
        if let Some(grounded) = ecs.get_mut::<Grounded>(entity) {
            grounded.0 = sweep.blocked_y && was_falling;
        }
    }
}

fn update_motion_bodies(ecs: &mut Ecs, dt: f32) {
    let entities: Vec<_> = ecs
        .get_store::<MotionBody>()
        .data
        .keys()
        .filter(|entity| !ecs.has::<PhysicsBody>(**entity))
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

    fn empty_room() -> Room {
        Room {
            variants: vec![RoomVariant::default()],
            ..Default::default()
        }
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

        update_physics(
            &SpriteManager::default(),
            &mut ecs,
            &empty_room(),
            1.0 / 60.0,
            16.0,
        );

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

        update_physics(
            &SpriteManager::default(),
            &mut ecs,
            &empty_room(),
            1.0 / 60.0,
            16.0,
        );

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
            .with(SubPixel::default())
            .finish();

        update_physics(
            &SpriteManager::default(),
            &mut ecs,
            &empty_room(),
            1.0 / 60.0,
            16.0,
        );

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
}
