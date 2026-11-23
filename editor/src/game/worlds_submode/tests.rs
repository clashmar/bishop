use super::{PointerDownAction, apply_marquee, handle_pointer_down, world_rect};
use crate::game::world_selection::WorldSelection;
use bishop::prelude::{vec2, Rect};
use engine_core::worlds::WorldId;

#[test]
fn worlds_submode_world_rect_uses_position_and_sprite_dimensions() {
    let position = vec2(12.0, 24.0);
    let size = vec2(80.0, 60.0);

    assert_eq!(world_rect(position, size), Rect::new(12.0, 24.0, 80.0, 60.0));
}

#[test]
fn worlds_submode_plain_click_on_world_selects_and_starts_drag() {
    let mut selection = WorldSelection::default();
    let world = WorldId(1);

    let action = handle_pointer_down(&mut selection, Some(world), false);

    assert_eq!(action, PointerDownAction::StartDrag(world));
    assert_eq!(selection.primary(), Some(world));
}

#[test]
fn worlds_submode_plain_click_empty_clears_and_starts_marquee() {
    let mut selection = WorldSelection::default();
    selection.select_only(WorldId(1));

    let action = handle_pointer_down(&mut selection, None, false);

    assert_eq!(action, PointerDownAction::StartMarquee);
    assert!(!selection.contains(WorldId(1)));
}

#[test]
fn worlds_submode_shift_click_toggles_without_starting_drag() {
    let mut selection = WorldSelection::default();
    let world = WorldId(1);

    let action = handle_pointer_down(&mut selection, Some(world), true);

    assert_eq!(action, PointerDownAction::None);
    assert!(selection.contains(world));
}

#[test]
fn worlds_submode_click_selected_world_preserves_group_and_makes_it_primary() {
    let mut selection = WorldSelection::default();
    let first = WorldId(1);
    let second = WorldId(2);
    selection.select_only(first);
    selection.toggle(second);

    let action = handle_pointer_down(&mut selection, Some(first), false);

    assert_eq!(action, PointerDownAction::StartDrag(first));
    assert!(selection.contains(first));
    assert!(selection.contains(second));
    assert_eq!(selection.primary(), Some(first));
}

#[test]
fn worlds_submode_marquee_adds_hits_and_uses_smallest_new_hit_as_primary() {
    let mut selection = WorldSelection::default();
    let existing = WorldId(9);
    let hit_a = WorldId(3);
    let hit_b = WorldId(2);
    selection.select_only(existing);
    let bounds = [
        (hit_a, Rect::new(10.0, 10.0, 10.0, 10.0)),
        (hit_b, Rect::new(20.0, 20.0, 10.0, 10.0)),
    ];

    let changed = apply_marquee(
        &mut selection,
        vec2(0.0, 0.0),
        vec2(40.0, 40.0),
        vec2(0.0, 0.0),
        vec2(10.0, 10.0),
        &bounds,
    );

    assert!(changed);
    assert!(selection.contains(existing));
    assert!(selection.contains(hit_a));
    assert!(selection.contains(hit_b));
    assert_eq!(selection.primary(), Some(hit_b));
}

#[test]
fn worlds_submode_marquee_is_direction_independent_and_requires_drag_threshold() {
    let mut selection = WorldSelection::default();
    let hit = WorldId(1);
    let bounds = [(hit, Rect::new(10.0, 10.0, 10.0, 10.0))];
    let too_short = apply_marquee(
        &mut selection,
        vec2(30.0, 30.0),
        vec2(0.0, 0.0),
        vec2(30.0, 30.0),
        vec2(35.0, 30.0),
        &bounds,
    );

    assert!(!too_short);
    assert!(!selection.contains(hit));

    let changed = apply_marquee(
        &mut selection,
        vec2(30.0, 30.0),
        vec2(0.0, 0.0),
        vec2(30.0, 30.0),
        vec2(40.0, 30.0),
        &bounds,
    );

    assert!(changed);
    assert!(selection.contains(hit));
}

#[test]
fn worlds_submode_marquee_with_no_intersections_preserves_selection() {
    let mut selection = WorldSelection::default();
    let selected = WorldId(1);
    selection.select_only(selected);
    let bounds = [(WorldId(2), Rect::new(100.0, 100.0, 10.0, 10.0))];

    let changed = apply_marquee(
        &mut selection,
        vec2(0.0, 0.0),
        vec2(10.0, 10.0),
        vec2(0.0, 0.0),
        vec2(10.0, 10.0),
        &bounds,
    );

    assert!(!changed);
    assert!(selection.contains(selected));
    assert_eq!(selection.primary(), Some(selected));
}
