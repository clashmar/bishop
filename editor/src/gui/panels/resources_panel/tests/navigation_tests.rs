use super::super::navigation::Navigation;

#[test]
fn navigation_starts_at_root() {
    let nav = Navigation::new();
    assert!(nav.is_at_root());
}

#[test]
fn navigation_push_goes_into_subdirectory() {
    let mut nav = Navigation::new();
    nav.push("assets");
    assert!(!nav.is_at_root());
}

#[test]
fn navigation_pop_goes_back_to_parent() {
    let mut nav = Navigation::new();
    nav.push("assets");
    let went_back = nav.pop();
    assert!(went_back);
    assert!(nav.is_at_root());
}

#[test]
fn navigation_pop_at_root_returns_false() {
    let mut nav = Navigation::new();
    let went_back = nav.pop();
    assert!(!went_back);
    assert!(nav.is_at_root());
}

#[test]
fn navigation_deep_path_push_pop() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.push("tiles");
    assert!(!nav.is_at_root());
    nav.pop();
    assert!(!nav.is_at_root());
    nav.pop();
    assert!(!nav.is_at_root());
    nav.pop();
    assert!(nav.is_at_root());
}

#[test]
fn navigation_depth_reflects_segment_count() {
    let mut nav = Navigation::new();
    assert_eq!(nav.depth(), 0);
    nav.push("assets");
    assert_eq!(nav.depth(), 1);
    nav.push("sprites");
    assert_eq!(nav.depth(), 2);
    nav.pop();
    assert_eq!(nav.depth(), 1);
}

#[test]
fn navigation_truncate_to_root() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.push("tiles");
    nav.truncate_to(0);
    assert!(nav.is_at_root());
    assert_eq!(nav.depth(), 0);
}

#[test]
fn navigation_truncate_to_mid_depth() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.push("tiles");
    nav.truncate_to(1);
    assert_eq!(nav.depth(), 1);
    assert_eq!(nav.segment(0), Some("assets"));
}

#[test]
fn navigation_truncate_to_current_depth_is_noop() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.truncate_to(2);
    assert_eq!(nav.depth(), 2);
}

#[test]
fn navigation_segment_returns_correct_value() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    assert_eq!(nav.segment(0), Some("assets"));
    assert_eq!(nav.segment(1), Some("sprites"));
    assert_eq!(nav.segment(2), None);
}
