// engine_core/src/animation/animation_system.rs
use crate::animation::{ClipId, resolve_sprite_id};
use crate::assets::AssetRegistry;
use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::{Animation, CurrentFrame, SpriteId};
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::PlayerProxy;
use crate::rendering::render_room::pivot_adjusted_position;
use crate::rendering::renderable::{EntityDrawParams, Renderable};
use crate::worlds::room::RoomId;
use crate::worlds::room::entities_in_room;
use bishop::prelude::*;
use std::collections::HashSet;

pub fn update_animation_sytem(
    loader: &impl TextureLoader,
    ecs: &mut Ecs,
    asset_registry: &mut AssetRegistry,
    sprite_manager: &mut SpriteManager,
    dt: f32,
    room_id: RoomId,
) {
    // Gather the ids of all entities that are in the current room
    let mut entities = entities_in_room(ecs, room_id);

    // Process the player entity if there's a player proxy
    let has_spawn_point = entities.iter().any(|e| ecs.has::<PlayerProxy>(*e));
    if has_spawn_point && let Some(player) = ecs.get_player_entity() {
        entities.insert(player);
    }

    update_entity_animations(loader, ecs, asset_registry, sprite_manager, dt, &entities);
}

/// Updates animation state for the supplied entities.
pub fn update_entity_animations(
    loader: &impl TextureLoader,
    ecs: &mut Ecs,
    asset_registry: &mut AssetRegistry,
    sprite_manager: &mut SpriteManager,
    dt: f32,
    entities: &HashSet<Entity>,
) {
    let anim_store = ecs.get_store_mut::<Animation>();

    let mut frames: Vec<(Entity, CurrentFrame)> = vec![];
    let mut to_remove: Vec<Entity> = vec![];

    for (entity, animation) in anim_store.data.iter_mut() {
        if !entities.contains(entity) {
            continue;
        }

        // Bail out early if there is no active clip.
        let Some(current_id) = &animation.current.clone() else {
            to_remove.push(*entity);
            continue;
        };

        // Get the sprite id
        let (sprite_id, resolved) =
            get_sprite_id(loader, animation, current_id, asset_registry, sprite_manager);

        if resolved {
            animation.update_cache_entry(current_id, sprite_id, sprite_manager);
        }

        let Some(clip) = animation.clips.get(current_id) else {
            continue;
        };
        let clip_state = animation.states.get_mut(current_id).unwrap();

        // Advance the timer with speed multiplier applied (0.0 means default speed of 1.0)
        let speed = if animation.speed_multiplier == 0.0 {
            1.0
        } else {
            animation.speed_multiplier
        };
        clip_state.timer += dt * speed;

        loop {
            let frame_index = clip_state.row * clip.cols + clip_state.col;
            let frame_time = if !clip.frame_durations.is_empty() {
                clip.frame_durations
                    .get(frame_index)
                    .copied()
                    .unwrap_or(1.0 / clip.fps.max(0.001))
            } else {
                1.0 / clip.fps.max(0.001)
            };

            if clip_state.timer < frame_time {
                break;
            }

            clip_state.timer -= frame_time;
            clip_state.col += 1;
            if clip_state.col >= clip.cols {
                clip_state.col = 0;
                clip_state.row += 1;
                if clip_state.row >= clip.rows {
                    if clip.looping {
                        clip_state.row = 0;
                    } else {
                        clip_state.finished = true;
                        clip_state.row = clip.rows - 1;
                        clip_state.col = clip.cols - 1;
                        break;
                    }
                }
            }
        }

        let frame = CurrentFrame {
            clip_id: animation.current.clone().unwrap(),
            col: clip_state.col,
            row: clip_state.row,
            offset: clip.offset,
            sprite_id,
            frame_size: clip.frame_size,
            flip_x: animation.flip_x,
        };

        frames.push((*entity, frame));
    }

    for (entity, frame) in frames {
        ecs.add_component_to_entity(entity, frame)
    }

    // Remove stale CurrentFrame components from entities with no active clip
    let frame_store = ecs.get_store_mut::<CurrentFrame>();
    for entity in to_remove {
        frame_store.remove(entity);
    }
}

impl Renderable for CurrentFrame {
    fn dimensions(&self, _sprite_manager: &SpriteManager) -> Option<Vec2> {
        Some(self.frame_size)
    }

    fn draw<C: BishopContext>(
        &self,
        ctx: &mut C,
        sprite_manager: &mut SpriteManager,
        params: &EntityDrawParams,
    ) -> bool {
        if self.sprite_id.0 == 0 {
            return false;
        }
        let tex = sprite_manager.get_texture_from_id(ctx, self.sprite_id);
        let frame_w = self.frame_size.x;
        let frame_h = self.frame_size.y;
        let src = Rect::new(
            self.col as f32 * frame_w,
            self.row as f32 * frame_h,
            frame_w,
            frame_h,
        );
        let draw_base = pivot_adjusted_position(params.pos, self.frame_size, params.pivot);
        let draw_x = (draw_base.x + self.offset.x).floor();
        let draw_y = (draw_base.y + self.offset.y).floor();
        ctx.draw_texture_ex(
            tex,
            draw_x,
            draw_y,
            Color::WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(frame_w, frame_h)),
                source: Some(src),
                flip_x: self.flip_x,
                ..Default::default()
            },
        );
        true
    }
}

/// Return the SpriteId for for the current animation clip.
fn get_sprite_id(
    loader: &impl TextureLoader,
    animation: &Animation,
    current_id: &ClipId,
    asset_registry: &mut AssetRegistry,
    sprite_manager: &mut SpriteManager,
) -> (SpriteId, bool) {
    if let Some(&cached) = animation.sprite_cache.get(current_id)
        && cached.0 != 0
    {
        return (cached, false);
    }

    let resolved = resolve_sprite_id(
        loader,
        asset_registry,
        sprite_manager,
        &animation.variant,
        current_id,
    );

    (resolved, true)
}
