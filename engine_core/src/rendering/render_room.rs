// NOTE: Multi-pass rendering temporarily disabled while rewiring codebase.
use bishop::prelude::*;
use crate::ecs::*;
use crate::ecs::components::transform::Pivot;
use crate::rendering::*;
use crate::worlds::*;
use crate::assets::*;
use crate::game::*;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Default)]
pub struct LayerData<'a> {
    pub entities: Vec<(Entity, Vec2)>,
    pub glows: Vec<(&'a Glow, Vec2)>,
}

/// Draws everything needed for the given room.
/// Currently uses simplified single-pass rendering.
pub fn render_room<C: BishopContext>(
    ctx: &mut C,
    game_ctx: &mut GameCtxMut<'_>,
    render_cam: &Camera2D,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
) {
    let Some(world) = game_ctx.world.as_deref_mut() else {
        return;
    };
    let Some(current_room) = world.current_room() else {
        return;
    };

    let grid_size = world.grid_size;

    // Organize entities by layer
    let layer_map = collect_interpolated_layer_map(
        game_ctx.ecs,
        world,
        current_room,
        game_ctx.sprite_manager,
        alpha,
        prev_positions,
        grid_size,
    );

    // Set up camera and clear background
    ctx.set_camera(render_cam);
    ctx.clear_background(Color::BLACK);

    // Draw tilemap first
    let tilemap = &current_room.current_variant().tilemap;
    tilemap.draw(
        ctx,
        game_ctx.sprite_manager,
        current_room.position,
        grid_size,
    );

    // Draw all entities sorted by layer
    for (_z, layer) in layer_map {
        for (entity, pos) in layer.entities {
            draw_entity(
                ctx,
                game_ctx.ecs,
                game_ctx.sprite_manager,
                entity,
                pos,
                grid_size,
            );
        }

        // TODO: Re-enable multi-pass rendering
        // render_system.run_ambient_pass(ctx, room.darkness);
        // render_system.run_glow_pass(ctx, render_cam, glows, sprite_manager);
        // render_system.run_undarkened_pass(ctx);
        // render_system.run_scene_pass(ctx);
    }

    // TODO: Re-enable multi-pass rendering
    // let lights = collect_lights(ecs, room, alpha, prev_positions);
    // render_system.run_spotlight_pass(ctx, render_cam, lights, room.darkness);
    // render_system.run_final_pass(ctx);
}

fn draw_entity<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    sprite_manager: &mut SpriteManager,
    entity: Entity,
    pos: Vec2,
    grid_size: f32,
) {
    let visual_entity = resolve_visual_entity(ecs, entity);

    let pivot = ecs
        .get_store::<Transform>()
        .get(entity)
        .map(|t| t.pivot)
        .unwrap_or(Pivot::BottomCenter);

    let params = EntityDrawParams {
        pos,
        pivot,
        grid_size,
    };

    if let Some(cf) = ecs.get_store::<CurrentFrame>().get(visual_entity)
        && cf.draw(ctx, sprite_manager, &params)
    {
        return;
    }

    if let Some(sprite) = ecs.get_store::<Sprite>().get(visual_entity)
        && sprite.draw(ctx, sprite_manager, &params)
    {
        return;
    }
}

/// Collects current-room entities plus neighboring spillover entities whose visual
/// bounds still overlap, sorted by z-layer with interpolated draw positions.
fn collect_interpolated_layer_map<'a>(
    ecs: &'a Ecs,
    world: &World,
    room: &Room,
    sprite_manager: &SpriteManager,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
    grid_size: f32,
) -> BTreeMap<i32, LayerData<'a>> {
    let mut map: BTreeMap<i32, LayerData<'a>> = BTreeMap::new();
    let mut seen = HashSet::new();

    let trans_store = ecs.get_store::<Transform>();
    let cam_store = ecs.get_store::<RoomCamera>();
    let layer_store = ecs.get_store::<Layer>();
    let glow_store = ecs.get_store::<Glow>();
    let sub_pixel_store = ecs.get_store::<SubPixel>();

    for candidate_room_id in spillover_candidate_room_ids(world, room) {
        for &entity in ecs.entities_in_room(candidate_room_id) {
            ecs.assert_room_membership(candidate_room_id, entity);

            if !seen.insert(entity) {
                continue;
            }

            let Some(transform) = trans_store.get(entity) else {
                continue;
            };

            if !transform.visible {
                continue;
            }

            if cam_store.get(entity).is_some() {
                continue;
            }

            let current_pos = visual_position(transform.position, sub_pixel_store.get(entity));
            let draw_pos = interpolate_position(entity, current_pos, alpha, prev_positions);

            if !entity_visible_in_room(
                ecs,
                sprite_manager,
                world,
                entity,
                candidate_room_id,
                draw_pos,
                room,
                grid_size,
            ) {
                continue;
            }

            let z = layer_store.get(entity).map_or(0, |layer| layer.z);

            let entry = map.entry(z).or_default();
            entry.entities.push((entity, draw_pos));

            if let Some(glow) = glow_store.get(entity) {
                let glow_size = sprite_manager
                    .texture_size(glow.sprite_id)
                    .map(|(w, h)| Vec2::new(w, h))
                    .unwrap_or(Vec2::new(grid_size, grid_size));

                let glow_draw_pos = pivot_adjusted_position(draw_pos, glow_size, transform.pivot);
                entry.glows.push((glow, glow_draw_pos));
            }
        }
    }

    // There always needs to be at least one layer otherwise nothing will be drawn
    if map.is_empty() {
        map.insert(0, LayerData::default());
    }

    map
}

// TODO: Re-enable for multi-pass rendering
// fn collect_lights(
//     ecs: &Ecs,
//     room: &Room,
//     alpha: f32,
//     prev_positions: Option<&HashMap<Entity, Vec2>>,
// ) -> Vec<(Vec2, Light)> { ... }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::test_support::make_vertical_spillover_fixture;
    use crate::worlds::test_utils::make_room;
    use crate::worlds::WorldId;

    #[test]
    fn collect_interpolated_layer_map_skips_entities_outside_the_room_index() {
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

        let layers = collect_interpolated_layer_map(
            &ecs,
            &world,
            world.get_room(room_id).unwrap(),
            &SpriteManager::default(),
            1.0,
            None,
            16.0,
        );

        assert!(layers
            .values()
            .flat_map(|layer| layer.entities.iter())
            .any(|(entity, _)| *entity == visible));
    }

    #[test]
    fn cross_room_visibility_collect_interpolated_layer_map_excludes_other_room_entity_at_non_exit_boundary() {
        let (world, ecs, entity, room_id) = make_vertical_spillover_fixture(vec2(32.0, 64.0), None);

        let layers = collect_interpolated_layer_map(
            &ecs,
            &world,
            world.get_room(room_id).unwrap(),
            &SpriteManager::default(),
            1.0,
            None,
            16.0,
        );

        assert!(!layers.values().flat_map(|layer| layer.entities.iter()).any(|(id, _)| *id == entity));
    }

    #[test]
    fn cross_room_visibility_collect_interpolated_layer_map_includes_other_room_entity_through_exit_cell() {
        let (world, ecs, entity, room_id) = make_vertical_spillover_fixture(
            vec2(32.0, 64.0),
            Some(Exit {
                position: vec2(1.0, 4.0),
                direction: ExitDirection::Down,
                target_room_id: Some(RoomId(2)),
            }),
        );

        let layers = collect_interpolated_layer_map(
            &ecs,
            &world,
            world.get_room(room_id).unwrap(),
            &SpriteManager::default(),
            1.0,
            None,
            16.0,
        );

        assert!(layers.values().flat_map(|layer| layer.entities.iter()).any(|(id, _)| *id == entity));
    }

    #[test]
    fn cross_room_visibility_collect_interpolated_layer_map_excludes_other_room_entity_once_fully_outside() {
        let (world, ecs, entity, room_id) = make_vertical_spillover_fixture(vec2(32.0, 96.0), None);

        let layers = collect_interpolated_layer_map(
            &ecs,
            &world,
            world.get_room(room_id).unwrap(),
            &SpriteManager::default(),
            1.0,
            None,
            16.0,
        );

        assert!(!layers.values().flat_map(|layer| layer.entities.iter()).any(|(id, _)| *id == entity));
    }
}
