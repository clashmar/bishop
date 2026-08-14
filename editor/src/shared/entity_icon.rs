use crate::editor_assets::assets::camera_icon;
use bishop::prelude::*;
use engine_core::assets::SpriteManager;
use engine_core::ecs::components::{
    Animation, CurrentFrame, Glow, Light, RoomCamera, Sprite, WorldEntry, WorldExit,
};
use engine_core::ecs::entity::Entity;
use engine_core::ecs::Ecs;
use engine_core::rendering::{outline_thickness, resolve_visual_entity};

pub(crate) const PLACEHOLDER_OPACITY: f32 = 0.5;

/// Visual representation category for an entity in the editor viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityVisual {
    SpriteOrAnimation,
    CameraIcon,
    PortalIcon,
    EntryIcon,
    ExitIcon,
    LightPlaceholder,
    GlowPlaceholder,
    GenericPlaceholder,
}

/// Resolves which visual representation to use for an entity based on its components.
pub fn resolve_entity_visual(ecs: &Ecs, entity: Entity) -> EntityVisual {
    let visual_entity = resolve_visual_entity(ecs, entity);
    let sprite_store = ecs.get_store::<Sprite>();
    let frame_store = ecs.get_store::<CurrentFrame>();

    if sprite_store.get(visual_entity).is_some_and(|s| s.has_valid_asset())
        || frame_store.get(visual_entity).is_some_and(|f| f.has_valid_asset())
        || ecs.has::<Animation>(visual_entity)
    {
        return EntityVisual::SpriteOrAnimation;
    }

    if ecs.has::<RoomCamera>(visual_entity) {
        return EntityVisual::CameraIcon;
    }

    let has_entry = ecs.has::<WorldEntry>(visual_entity);
    let has_exit = ecs.has::<WorldExit>(visual_entity);

    match (has_entry, has_exit) {
        (true, true) => return EntityVisual::PortalIcon,
        (true, false) => return EntityVisual::EntryIcon,
        (false, true) => return EntityVisual::ExitIcon,
        (false, false) => {}
    }

    if ecs.has::<Light>(visual_entity) {
        return EntityVisual::LightPlaceholder;
    }

    if ecs.has::<Glow>(visual_entity) {
        return EntityVisual::GlowPlaceholder;
    }

    EntityVisual::GenericPlaceholder
}

/// Draws a camera icon centered on the given position.
pub fn draw_camera_icon<C: BishopContext>(ctx: &mut C, pos: Vec2, grid_size: f32) {
    let half = grid_size * 0.5;
    ctx.draw_texture_ex(
        camera_icon(),
        pos.x - half,
        pos.y - half,
        Color::new(1.0, 1.0, 1.0, PLACEHOLDER_OPACITY),
        DrawTextureParams {
            dest_size: Some(vec2(grid_size, grid_size)),
            ..Default::default()
        },
    );
}

/// Draws the light placeholder shape for an entity without a visual component.
pub(crate) fn draw_light_placeholder<C: BishopContext>(ctx: &mut C, pos: Vec2, grid_size: f32) {
    let half_tile = grid_size * 0.5;
    let body = Rect::new(pos.x - half_tile, pos.y - half_tile, grid_size, grid_size);
    let cyan = Color::new(0.0, 0.78, 0.78, PLACEHOLDER_OPACITY);
    let yellow = Color::new(0.94, 0.86, 0.0, PLACEHOLDER_OPACITY);
    ctx.draw_rectangle_lines(body.x, body.y, body.w, body.h, outline_thickness(grid_size), cyan);
    let lens_radius = grid_size * 0.2;
    let lens_center = vec2(body.x + body.w / 2.0, body.y + body.h / 2.0);
    ctx.draw_circle_lines(
        lens_center.x,
        lens_center.y,
        lens_radius,
        outline_thickness(grid_size) * 0.75,
        yellow,
    );
}

/// Draws the glow placeholder shape for an entity without a visual component.
pub(crate) fn draw_glow_placeholder<C: BishopContext>(
    ctx: &mut C,
    sprite_manager: &mut SpriteManager,
    ecs: &Ecs,
    entity: Entity,
    pos: Vec2,
    grid_size: f32,
) {
    let glow_store = ecs.get_store::<Glow>();
    let Some(glow) = glow_store.get(entity) else {
        return;
    };

    let mut pos = pos;
    if let Some((w, h)) = sprite_manager.texture_size(glow.sprite_id) {
        pos += vec2((w / 2.0) - grid_size / 2.0, (h / 2.0) - grid_size / 2.0);
    }

    let body = Rect::new(pos.x, pos.y, grid_size, grid_size);
    let cyan = Color::new(0.0, 0.78, 0.78, PLACEHOLDER_OPACITY);
    let yellow = Color::new(0.94, 0.86, 0.0, PLACEHOLDER_OPACITY);
    ctx.draw_rectangle_lines(body.x, body.y, body.w, body.h, outline_thickness(grid_size), cyan);
    let lens_radius = grid_size * 0.2;
    let lens_center = vec2(body.x + body.w / 2.0, body.y + body.h / 2.0);
    ctx.draw_circle_lines(
        lens_center.x,
        lens_center.y,
        lens_radius,
        outline_thickness(grid_size) * 0.75,
        yellow,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::components::Transform;
    use engine_core::ecs::{Player, PlayerProxy, SpriteId};

    fn make_ecs() -> Ecs {
        Ecs::default()
    }

    fn make_entity(ecs: &mut Ecs) -> Entity {
        ecs.create_entity()
            .with(Transform::default())
            .finish()
    }

    #[test]
    fn sprite_with_valid_asset_returns_sprite_or_animation() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, Sprite { sprite: SpriteId(1) });
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::SpriteOrAnimation));
    }

    #[test]
    fn current_frame_with_valid_asset_returns_sprite_or_animation() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, CurrentFrame {
            sprite_id: SpriteId(1),
            ..Default::default()
        });
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::SpriteOrAnimation));
    }

    #[test]
    fn room_camera_without_visual_returns_camera_icon() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, RoomCamera::default());
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::CameraIcon));
    }

    #[test]
    fn entry_and_exit_without_visual_returns_portal_icon() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, WorldEntry::default());
        ecs.insert_component(entity, WorldExit::default());
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::PortalIcon));
    }

    #[test]
    fn entry_only_without_visual_returns_entry_icon() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, WorldEntry::default());
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::EntryIcon));
    }

    #[test]
    fn exit_only_without_visual_returns_exit_icon() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, WorldExit::default());
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::ExitIcon));
    }

    #[test]
    fn light_without_visual_returns_light_placeholder() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, Light::default());
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::LightPlaceholder));
    }

    #[test]
    fn glow_without_visual_returns_glow_placeholder() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, Glow::default());
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::GlowPlaceholder));
    }

    #[test]
    fn no_special_components_returns_generic_placeholder() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::GenericPlaceholder));
    }

    #[test]
    fn sprite_with_camera_returns_sprite_or_animation() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, Sprite { sprite: SpriteId(1) });
        ecs.insert_component(entity, RoomCamera::default());
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::SpriteOrAnimation));
    }

    #[test]
    fn sprite_with_entry_returns_sprite_or_animation() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, Sprite { sprite: SpriteId(1) });
        ecs.insert_component(entity, WorldEntry::default());
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::SpriteOrAnimation));
    }

    #[test]
    fn animation_without_current_frame_returns_sprite_or_animation() {
        let mut ecs = make_ecs();
        let entity = make_entity(&mut ecs);
        ecs.insert_component(entity, Animation::default());
        let result = resolve_entity_visual(&ecs, entity);
        assert!(matches!(result, EntityVisual::SpriteOrAnimation));
    }

    #[test]
    fn player_proxy_uses_player_visuals() {
        let mut ecs = make_ecs();
        let _player = ecs
            .create_entity()
            .with(Player)
            .with(Transform::default())
            .with(Sprite { sprite: SpriteId(1) })
            .finish();
        let proxy = ecs
            .create_entity()
            .with(PlayerProxy)
            .with(Transform::default())
            .finish();

        let result = resolve_entity_visual(&ecs, proxy);
        assert!(matches!(result, EntityVisual::SpriteOrAnimation));
    }
}
