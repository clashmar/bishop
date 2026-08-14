use bishop::prelude::*;
use engine_core::worlds::room::Room;
use engine_core::worlds::{InteriorZone, InteriorZoneBounds, InteriorZoneId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteriorZoneConstraintViolation {
    Overlap,
    OutOfBounds,
}

pub(crate) fn interior_zones_for_variant(
    room: &Room,
    variant_index: usize,
) -> &[InteriorZone] {
    room.variants[variant_index]
        .layers
        .back
        .as_ref()
        .map(|back| back.interior_zones.as_slice())
        .unwrap_or(&[])
}

pub(crate) fn validate_zone_set(
    zones: &[InteriorZone],
    room_rect: Rect,
) -> Option<InteriorZoneConstraintViolation> {
    if !all_zones_fit_room(zones, room_rect) {
        return Some(InteriorZoneConstraintViolation::OutOfBounds);
    }

    for (index, zone) in zones.iter().enumerate() {
        if zones[index + 1..]
            .iter()
            .any(|other| zone_rects_overlap(zone.bounds, other.bounds))
        {
            return Some(InteriorZoneConstraintViolation::Overlap);
        }
    }

    None
}

pub(crate) fn all_zones_fit_room(zones: &[InteriorZone], room_rect: Rect) -> bool {
    zones.iter().all(|zone| zone_fits_room(zone.bounds, room_rect))
}

pub(crate) fn zone_constraint_message(
    violation: InteriorZoneConstraintViolation,
) -> &'static str {
    match violation {
        InteriorZoneConstraintViolation::Overlap => "Interior zones cannot overlap",
        InteriorZoneConstraintViolation::OutOfBounds => {
            "Interior zones must stay inside the room bounds"
        }
    }
}

pub(crate) fn try_set_zone_bounds(
    zones: &mut [InteriorZone],
    zone_id: InteriorZoneId,
    candidate_bounds: InteriorZoneBounds,
    room_rect: Rect,
) -> Result<(), InteriorZoneConstraintViolation> {
    let Some(index) = zones.iter().position(|zone| zone.id == zone_id) else {
        return Ok(());
    };

    let original_bounds = zones[index].bounds;
    zones[index].bounds = candidate_bounds;

    if let Some(violation) = validate_zone_set(zones, room_rect) {
        zones[index].bounds = original_bounds;
        return Err(violation);
    }

    Ok(())
}

fn zone_fits_room(bounds: InteriorZoneBounds, room_rect: Rect) -> bool {
    let rect = bounds.to_rect();
    rect.x >= room_rect.x
        && rect.y >= room_rect.y
        && rect.right() <= room_rect.right()
        && rect.bottom() <= room_rect.bottom()
}

fn zone_rects_overlap(a: InteriorZoneBounds, b: InteriorZoneBounds) -> bool {
    let a = a.to_rect();
    let b = b.to_rect();
    a.x < b.right() && a.right() > b.x && a.y < b.bottom() && a.bottom() > b.y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(id: u64, x: i32, y: i32, w: i32, h: i32) -> InteriorZone {
        InteriorZone {
            id: InteriorZoneId(id),
            bounds: InteriorZoneBounds::new(x, y, w, h),
        }
    }

    #[test]
    fn interior_zone_rects_that_touch_edges_are_allowed() {
        let room_rect = Rect::new(0.0, 0.0, 64.0, 64.0);
        let zones = vec![zone(1, 0, 0, 16, 16), zone(2, 16, 0, 16, 16)];

        assert_eq!(validate_zone_set(&zones, room_rect), None);
    }

    #[test]
    fn interior_zone_rects_with_shared_area_are_rejected() {
        let room_rect = Rect::new(0.0, 0.0, 64.0, 64.0);
        let zones = vec![zone(1, 0, 0, 16, 16), zone(2, 8, 0, 16, 16)];

        assert_eq!(
            validate_zone_set(&zones, room_rect),
            Some(InteriorZoneConstraintViolation::Overlap),
        );
    }

    #[test]
    fn interior_zone_set_detects_out_of_bounds_zone() {
        let room_rect = Rect::new(0.0, 0.0, 32.0, 32.0);
        let zones = vec![zone(1, 24, 0, 16, 16)];

        assert_eq!(
            validate_zone_set(&zones, room_rect),
            Some(InteriorZoneConstraintViolation::OutOfBounds),
        );
        assert!(!all_zones_fit_room(&zones, room_rect));
    }

    #[test]
    fn try_set_zone_bounds_rejects_overlap_and_keeps_previous_bounds() {
        let room_rect = Rect::new(0.0, 0.0, 64.0, 64.0);
        let mut zones = vec![zone(1, 0, 0, 16, 16), zone(2, 32, 0, 16, 16)];

        let result = try_set_zone_bounds(
            &mut zones,
            InteriorZoneId(2),
            InteriorZoneBounds::new(8, 0, 16, 16),
            room_rect,
        );

        assert_eq!(result, Err(InteriorZoneConstraintViolation::Overlap));
        assert_eq!(zones[1].bounds, InteriorZoneBounds::new(32, 0, 16, 16));
    }

    #[test]
    fn try_set_zone_bounds_allows_edge_adjacent_candidate() {
        let room_rect = Rect::new(0.0, 0.0, 64.0, 64.0);
        let mut zones = vec![zone(1, 0, 0, 16, 16), zone(2, 32, 0, 16, 16)];

        let result = try_set_zone_bounds(
            &mut zones,
            InteriorZoneId(2),
            InteriorZoneBounds::new(16, 0, 16, 16),
            room_rect,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(zones[1].bounds, InteriorZoneBounds::new(16, 0, 16, 16));
    }
}
