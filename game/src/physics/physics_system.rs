// game/src/physics/physics_system.rs
use crate::constants::GRAVITY;
use crate::physics::collision::SweepContext;
use engine_core::prelude::*;

/// Applies physics to all entities with a `PhysicsBody` component.
pub fn update_physics(
    asset_manager: &AssetManager,
    ecs: &mut Ecs,
    room: &Room,
    dt: f32,
    grid_size: f32,
) {
    let tilemap = &room.variants[room.current_variant_index()].tilemap;

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
            asset_manager,
            ecs,
            tilemap,
            room.position,
            &room.exits,
            grid_size,
        );
        let sweep = collision_world.sweep_move(true_pos, delta, collider, pivot);

        // Snap to integer positions, storing the fractional part for next frame
        let new_true_pos = true_pos + sweep.allowed_delta;
        let new_int_pos = new_true_pos.round();

        sub_pixel.x = new_true_pos.x - new_int_pos.x;
        sub_pixel.y = new_true_pos.y - new_int_pos.y;

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
