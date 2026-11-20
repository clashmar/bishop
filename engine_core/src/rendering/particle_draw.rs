use crate::assets::SpriteManager;
use crate::ecs::components::{CurrentRoom, ParticleVisual};
use crate::ecs::Ecs;
use crate::rendering::particles::Particle;
use crate::rendering::room_composition::RoomCompositionContext;
use crate::worlds::RoomLayer;
use bishop::prelude::{BishopContext, Color, DrawTextureParams, Rect, Vec2};

/// Draws one particle using the active room composition and current camera.
pub(crate) fn draw_particle<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    sprite_manager: &mut SpriteManager,
    particle: &Particle,
    bounds: Rect,
    composition: &RoomCompositionContext,
) {
    let Some(current_room) = ecs.get::<CurrentRoom>(particle.owner) else {
        return;
    };
    let composition_alpha = match current_room.layer {
        RoomLayer::Front => {
            let Some(tint) = composition
                .front_layer_composition(ecs, particle.owner, Some(bounds))
                .tint()
            else {
                return;
            };
            tint.a
        }
        RoomLayer::Back => {
            if !composition.back_layer_bounds_visible(bounds) {
                return;
            }
            1.0
        }
    };
    let color = Color::new(
        particle.color.r,
        particle.color.g,
        particle.color.b,
        particle.color.a * particle.opacity * composition_alpha,
    );

    match &particle.visual {
        ParticleVisual::Pixel => ctx.draw_rectangle(
            particle.position.x,
            particle.position.y,
            particle.scale,
            particle.scale,
            color,
        ),
        ParticleVisual::Line => {
            let end = particle_line_end(particle);
            ctx.draw_line(
                particle.position.x,
                particle.position.y,
                end.x,
                end.y,
                particle.scale,
                color,
            );
        }
        ParticleVisual::Sprite { frame } => {
            if !frame.has_valid_asset() {
                return;
            }
            let texture = sprite_manager.get_texture_from_id(ctx, frame.sprite_id);
            let source = Rect::new(
                frame.col as f32 * frame.frame_size.x,
                frame.row as f32 * frame.frame_size.y,
                frame.frame_size.x,
                frame.frame_size.y,
            );
            let dest_size = frame.frame_size * particle.scale;
            let position = particle.position + frame.offset * particle.scale;
            ctx.draw_texture_ex(
                texture,
                position.x,
                position.y,
                color,
                DrawTextureParams {
                    dest_size: Some(dest_size),
                    source: Some(source),
                    flip_x: frame.flip_x,
                    ..Default::default()
                },
            );
        }
    }
}

fn particle_line_end(particle: &Particle) -> Vec2 {
    let speed = particle.velocity.x.hypot(particle.velocity.y);
    let direction = if speed > 0.0 && speed.is_finite() {
        particle.velocity / speed
    } else {
        Vec2::new(1.0, 0.0)
    };
    particle.position + direction * particle.line_length as f32 * particle.scale
}

/// Computes a particle's world-space draw bounds for room visibility and composition.
pub(crate) fn particle_visual_bounds(particle: &Particle) -> Rect {
    match &particle.visual {
        ParticleVisual::Pixel => Rect::new(
            particle.position.x,
            particle.position.y,
            particle.scale,
            particle.scale,
        ),
        ParticleVisual::Line => {
            let end = particle_line_end(particle);
            let half_thickness = particle.scale * 0.5;
            let min_x = particle.position.x.min(end.x) - half_thickness;
            let min_y = particle.position.y.min(end.y) - half_thickness;
            Rect::new(
                min_x,
                min_y,
                (particle.position.x - end.x).abs() + particle.scale,
                (particle.position.y - end.y).abs() + particle.scale,
            )
        }
        ParticleVisual::Sprite { frame } => {
            let size = frame.frame_size * particle.scale;
            let position = particle.position + frame.offset * particle.scale;
            Rect::new(position.x, position.y, size.x, size.y)
        }
    }
}
