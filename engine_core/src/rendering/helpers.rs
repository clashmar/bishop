use crate::prelude::*;
use bishop::prelude::*;

/// Resolves the entity to use for visual lookups. A `PlayerProxy` redirects
/// to the actual player entity so the proxy renders with the player's visuals.
pub fn resolve_visual_entity(ecs: &Ecs, entity: Entity) -> Entity {
    if ecs.has::<PlayerProxy>(entity) {
        ecs.get_player_entity().unwrap_or(entity)
    } else {
        entity
    }
}

/// Returns the pixel dimensions of an entity for rendering.
pub fn entity_dimensions(
    ecs: &Ecs,
    asset_manager: &AssetManager,
    entity: Entity,
    grid_size: f32,
) -> Vec2 {
    let visual_entity = resolve_visual_entity(ecs, entity);
    let from_anim = ecs
        .get_store::<CurrentFrame>()
        .get(visual_entity)
        .and_then(|cf| cf.dimensions(asset_manager));

    let from_sprite = || {
        ecs.get_store::<Sprite>()
            .get(visual_entity)
            .and_then(|s| s.dimensions(asset_manager))
    };

    from_anim
        .or_else(from_sprite)
        .unwrap_or(Vec2::splat(grid_size))
}

/// Linearly interpolates between two positions and rounds to the nearest pixel.
#[inline]
pub fn lerp_rounded(prev_pos: Vec2, current_pos: Vec2, alpha: f32) -> Vec2 {
    (prev_pos * (1.0 - alpha) + current_pos * alpha).round()
}

/// Mitigates erratic dt by smoothing `raw_dt`, initializing from the first sample.
/// `alpha` is the weight of the previous smoothed value (higher = smoother but slower to react).
#[inline]
pub fn smooth_dt(smoothed_dt: &mut Option<f32>, raw_dt: f32, alpha: f32) -> f32 {
    let s = smoothed_dt.get_or_insert(raw_dt);
    *s = *s * alpha + raw_dt * (1.0 - alpha);
    *s
}

/// Common display refresh rates to snap frame times to (checked in order).
const SNAP_FREQUENCIES: [f32; 5] = [60.0, 120.0, 144.0, 240.0, 30.0];

/// Snaps raw_dt to the nearest common display interval if within 10% of it.
/// Eliminates accumulator drift that causes periodic stutter.
#[inline]
pub fn snap_dt(raw_dt: f32) -> f32 {
    for freq in SNAP_FREQUENCIES {
        let target = 1.0 / freq;
        if (raw_dt - target).abs() < target * 0.1 {
            return target;
        }
    }
    raw_dt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_visual_entity_returns_player_for_proxy() {
        let mut ecs = Ecs::default();
        let player = ecs.create_entity().with(Player).finish();
        let proxy = ecs.create_entity().with(PlayerProxy).finish();

        assert_eq!(resolve_visual_entity(&ecs, proxy), player);
    }

    #[test]
    fn entity_dimensions_use_player_visuals_for_proxy() {
        let mut ecs = Ecs::default();
        let player = ecs
            .create_entity()
            .with(Player)
            .with(CurrentFrame {
                frame_size: vec2(6.0, 16.0),
                ..Default::default()
            })
            .finish();
        let proxy = ecs.create_entity().with(PlayerProxy).finish();
        let asset_manager = AssetManager::default();

        assert_eq!(resolve_visual_entity(&ecs, proxy), player);
        assert_eq!(
            entity_dimensions(&ecs, &asset_manager, proxy, 8.0),
            vec2(6.0, 16.0),
        );
    }
}
