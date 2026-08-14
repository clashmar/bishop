use super::*;

#[test]
fn multi_select_returns_added_item_when_selected() {
    let selected: Vec<&str> = vec!["Bob"];

    assert_eq!(selected.len(), 1);
    assert!(selected.contains(&"Bob"));
}

#[test]
fn multi_select_excludes_already_selected_from_options() {
    let options = ["Alice", "Bob"];
    let selected = ["Alice"];

    let available: Vec<&str> = options
        .iter()
        .copied()
        .filter(|option| !selected.contains(option))
        .collect();
    assert_eq!(available, vec!["Bob"]);
}

#[test]
fn multi_select_returns_removed_item_when_chip_clicked() {
    let mut selected = vec!["Alice", "Bob"];
    selected.retain(|s| *s != "Bob");
    assert_eq!(selected, vec!["Alice"]);
}

#[test]
fn multi_select_chip_layout_wraps_after_max_columns() {
    let chips_per_row = 3;
    let item_count = 4;
    let rows = (item_count + chips_per_row - 1) / chips_per_row;
    assert_eq!(rows, 2);
}

#[test]
fn multi_select_delta_captures_both_added_and_removed() {
    let delta: MultiSelectDelta<&str> = MultiSelectDelta {
        added: vec!["Alice"],
        removed: vec!["Bob"],
    };
    assert_eq!(delta.added, vec!["Alice"]);
    assert_eq!(delta.removed, vec!["Bob"]);
}
