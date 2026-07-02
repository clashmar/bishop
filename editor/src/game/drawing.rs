use std::collections::HashMap;

use bishop::prelude::*;
use engine_core::{ecs::SpriteId, game::Game, worlds::{WorldId, extract_topology}};
use widgets::{center_text_field, draw_input_field_text, with_theme};

use crate::{app::SubEditor, editor_assets::assets::circle_120px, game::game_editor::{GameEditor, GameEditorMode}, gui::gui_constants::SPACING, world::coord};

impl GameEditor {
    pub(crate) fn draw_worlds(&mut self, ctx: &mut WgpuContext, camera: &Camera2D, game: &mut Game) {
        let world_data: Vec<(WorldId, Vec2, Option<SpriteId>, String)> = game
            .worlds()
            .iter()
            .map(|w| (w.id, w.meta.position, w.meta.sprite_id, w.name.clone()))
            .collect();

        if self.show_world_arrows {
            Self::draw_world_arrows(ctx, game, &world_data);
        }

        for (_world_id, position, sprite_id, name) in world_data {
            let texture = match sprite_id {
                Some(id) => game.sprite_manager.get_texture_from_id(ctx, id),
                None => circle_120px(),
            };

            // Hover tint — inline the bounds check
            let world_mouse = coord::mouse_world_pos(ctx, camera);
            let bounds = Rect::new(
                position.x,
                position.y,
                texture.width(),
                texture.height(),
            );
            let is_hovered = bounds.contains(world_mouse)
                && !self.should_block_canvas(ctx)
                && self.dragged_world.is_none();

            let tint = if is_hovered {
                match self.mode {
                    GameEditorMode::Delete => with_theme(|t| t.danger),
                    _ => with_theme(|t| t.primary),
                }
            } else {
                Color::WHITE
            };

            // Default is a circle
            ctx.draw_texture(texture, position.x, position.y, tint);

            // Display name
            const NAME_HEIGHT: f32 = 24.0;
            let center = position.x + (texture.width() / 2.);
            let (x, width) = center_text_field(ctx, center, &name);

            let name_rect = Rect::new(
                x,
                position.y - SPACING - NAME_HEIGHT,
                width,
                NAME_HEIGHT,
            );

            draw_input_field_text(ctx, &name, name_rect);
        }
    }

    pub(crate) fn draw_world_arrows(
        ctx: &mut WgpuContext,
        game: &mut Game,
        world_data: &[(WorldId, Vec2, Option<SpriteId>, String)],
    ) {
        let topology = extract_topology(game);
        let bounds: HashMap<WorldId, Rect> = world_data
            .iter()
            .map(|(world_id, position, sprite_id, _name)| {
                let texture = match sprite_id {
                    Some(id) => game.sprite_manager.get_texture_from_id(ctx, *id),
                    None => circle_120px(),
                };
                (
                    *world_id,
                    Rect::new(position.x, position.y, texture.width(), texture.height()),
                )
            })
            .collect();

        for (&from_world, from_bounds) in &bounds {
            let from_center = from_bounds.center();
            for edge in topology.world_graph.edges_from(from_world) {
                let Some(to_bounds) = bounds.get(&edge.to) else {
                    continue;
                };
                let to_center = to_bounds.center();
                let delta = to_center - from_center;
                if delta.length_squared() == 0.0 {
                    continue;
                }
                let direction = delta.normalize();
                let from = rect_edge_point(*from_bounds, from_center, direction);
                let to = rect_edge_point(*to_bounds, to_center, -direction);
                draw_directed_arrow(ctx, from, to, with_theme(|theme| theme.accent));
            }
        }
    }
}

pub(crate) fn draw_directed_arrow(ctx: &mut WgpuContext, from: Vec2, to: Vec2, color: Color) {
    let delta = to - from;
    if delta.length_squared() == 0.0 {
        return;
    }

    let direction = delta.normalize();
    let tip = to;
    let shaft_end = to - direction * 12.0;
    let normal = vec2(-direction.y, direction.x) * 5.0;

    ctx.draw_line(from.x, from.y, shaft_end.x, shaft_end.y, 2.0, color);
    ctx.draw_triangle(tip, shaft_end + normal, shaft_end - normal, color);
}

pub(crate) fn rect_edge_point(bounds: Rect, center: Vec2, direction: Vec2) -> Vec2 {
    let half = vec2(bounds.w * 0.5, bounds.h * 0.5);
    let scale_x = if direction.x.abs() > f32::EPSILON {
        half.x / direction.x.abs()
    } else {
        f32::INFINITY
    };
    let scale_y = if direction.y.abs() > f32::EPSILON {
        half.y / direction.y.abs()
    } else {
        f32::INFINITY
    };
    let scale = scale_x.min(scale_y);
    center + direction * scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_edge_point_returns_target_edge_for_horizontal_direction() {
        let bounds = Rect::new(10.0, 20.0, 80.0, 40.0);
        let center = bounds.center();

        assert_eq!(rect_edge_point(bounds, center, vec2(1.0, 0.0)), vec2(90.0, 40.0));
        assert_eq!(rect_edge_point(bounds, center, vec2(-1.0, 0.0)), vec2(10.0, 40.0));
    }
}