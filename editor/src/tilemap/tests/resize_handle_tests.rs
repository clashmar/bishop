use crate::tilemap::resize_handle::{validate_resize, HandleSide, PreviewData, ResizeResult};
use bishop::prelude::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::{InteriorZone, InteriorZoneBounds, InteriorZoneId};

#[test]
fn validate_resize_when_shrink_would_cut_into_interior_zone_then_returns_zone_bounds_result() {
    let map = TileMap::new(4, 4);
    let zones = vec![InteriorZone {
        id: InteriorZoneId(1),
        bounds: InteriorZoneBounds::new(16, 0, 16, 16),
    }];

    let result = validate_resize(
        &map,
        &[],
        HandleSide::Right,
        -3,
        &[],
        PreviewData::new(Vec2::ZERO, vec2(16.0, 64.0)),
        16.0,
        &zones,
    );

    assert_eq!(result, ResizeResult::InteriorZonesOutOfBounds);
}

#[test]
fn validate_resize_when_all_zones_stay_inside_preview_then_succeeds() {
    let map = TileMap::new(4, 4);
    let zones = vec![InteriorZone {
        id: InteriorZoneId(1),
        bounds: InteriorZoneBounds::new(0, 0, 16, 16),
    }];

    let result = validate_resize(
        &map,
        &[],
        HandleSide::Right,
        -2,
        &[],
        PreviewData::new(Vec2::ZERO, vec2(32.0, 64.0)),
        16.0,
        &zones,
    );

    assert_eq!(result, ResizeResult::Success);
}
