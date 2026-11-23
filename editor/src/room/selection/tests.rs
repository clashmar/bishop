use super::*;

fn create_room_member(ecs: &mut Ecs, room_id: RoomId, layer: RoomLayer) -> Entity {
    ecs.create_entity()
        .with(Transform::default())
        .with_current_room_layer(room_id, layer)
        .finish()
}

#[test]
fn room_cycle_candidates_preserve_sorted_multi_selection() {
    let mut editor = RoomEditor::new();
    let first = Entity(7);
    let second = Entity(3);
    let third = Entity(11);
    editor.selected_entities.extend([first, second, third]);

    assert_eq!(
        editor.cycle_entities_for_room(&Ecs::default(), RoomId(1)),
        vec![second, first, third]
    );
}

#[test]
fn room_cycle_candidates_include_all_layers_and_descendants_but_skip_singletons() {
    let room_id = RoomId(4);
    let mut ecs = Ecs::default();
    let front = create_room_member(&mut ecs, room_id, RoomLayer::Front);
    let back = create_room_member(&mut ecs, room_id, RoomLayer::Back);
    let child = ecs
        .create_entity()
        .with(Transform::default())
        .with(Parent(front))
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    ecs.create_entity()
        .with(Singleton)
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    let other_room = create_room_member(&mut ecs, RoomId(9), RoomLayer::Front);

    let mut editor = RoomEditor::new();
    editor.set_selected_entity(Some(front));
    let mut expected = vec![front, back, child];
    expected.sort_unstable();

    assert_eq!(editor.cycle_entities_for_room(&ecs, room_id), expected);
    assert!(!editor
        .cycle_entities_for_room(&ecs, room_id)
        .contains(&other_room));
}

#[test]
fn room_cycle_candidates_exclude_room_members_without_transforms() {
    let room_id = RoomId(3);
    let mut ecs = Ecs::default();
    let selected = create_room_member(&mut ecs, room_id, RoomLayer::Front);
    ecs.create_entity()
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    let mut editor = RoomEditor::new();
    editor.set_selected_entity(Some(selected));

    assert_eq!(editor.cycle_entities_for_room(&ecs, room_id), vec![selected]);
}

#[test]
fn room_cycle_candidates_keep_single_candidate_without_adding_singletons() {
    let room_id = RoomId(2);
    let mut ecs = Ecs::default();
    let selected = create_room_member(&mut ecs, room_id, RoomLayer::Front);
    ecs.create_entity()
        .with(Singleton)
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();
    let mut editor = RoomEditor::new();
    editor.set_selected_entity(Some(selected));

    assert_eq!(editor.cycle_entities_for_room(&ecs, room_id), vec![selected]);
}

#[test]
fn room_cycle_candidate_navigation_wraps_in_both_directions() {
    let room_id = RoomId(5);
    let mut ecs = Ecs::default();
    let first = create_room_member(&mut ecs, room_id, RoomLayer::Front);
    let second = create_room_member(&mut ecs, room_id, RoomLayer::Front);
    let last = create_room_member(&mut ecs, room_id, RoomLayer::Back);
    let mut editor = RoomEditor::new();
    editor.set_active_layer(&ecs, room_id, RoomLayer::Back);
    editor.set_selected_entity(Some(last));

    let candidates = editor.cycle_entities_for_room(&ecs, room_id);
    assert_eq!(candidates, vec![first, second, last]);
    editor.inspector.set_cycle_entities(candidates.clone());
    assert_eq!(
        editor.inspector.cycle_selected_entity(true),
        Some(candidates[0])
    );
    editor.apply_inspector_cycle_target(candidates[0], &ecs, room_id);
    editor.prune_selection_to_active_layer(&ecs, room_id);
    assert_eq!(editor.selected_entities, HashSet::from([first]));

    let candidates = editor.cycle_entities_for_room(&ecs, room_id);
    editor.inspector.set_cycle_entities(candidates);
    assert_eq!(editor.inspector.cycle_selected_entity(false), Some(last));
    editor.apply_inspector_cycle_target(last, &ecs, room_id);
    editor.prune_selection_to_active_layer(&ecs, room_id);
    assert_eq!(editor.selected_entities, HashSet::from([last]));
}

#[test]
fn room_cycle_target_updates_primary_without_changing_multi_selection() {
    let first = Entity(3);
    let second = Entity(7);
    let mut editor = RoomEditor::new();
    editor.selected_entities.extend([first, second]);
    editor.primary_selected_entity = Some(first);
    editor.inspector.select_entity(second);

    editor.apply_inspector_cycle_target(second, &Ecs::default(), RoomId(1));

    assert_eq!(editor.selected_entities, HashSet::from([first, second]));
    assert_eq!(editor.primary_selected_entity, Some(second));
    assert_eq!(editor.inspector.selected_entity(), Some(second));
}

#[test]
fn room_cycle_target_remains_selected_after_active_layer_pruning() {
    let room_id = RoomId(6);
    let mut ecs = Ecs::default();
    let front = create_room_member(&mut ecs, room_id, RoomLayer::Front);
    let back = create_room_member(&mut ecs, room_id, RoomLayer::Back);
    let mut editor = RoomEditor::new();
    editor.set_active_layer(&ecs, room_id, RoomLayer::Front);
    editor.set_selected_entity(Some(front));
    editor.inspector.select_entity(back);

    editor.apply_inspector_cycle_target(back, &ecs, room_id);
    editor.prune_selection_to_active_layer(&ecs, room_id);

    assert_eq!(editor.selected_entities, HashSet::from([back]));
    assert_eq!(editor.primary_selected_entity, Some(back));
    assert_eq!(editor.inspector.selected_entity(), Some(back));
    assert_eq!(editor.active_layer_state.active_layer, RoomLayer::Back);
}

#[test]
fn room_cycle_target_replaces_single_selection() {
    let current = Entity(3);
    let target = Entity(7);
    let mut editor = RoomEditor::new();
    editor.set_selected_entity(Some(current));
    editor.inspector.select_entity(target);

    editor.apply_inspector_cycle_target(target, &Ecs::default(), RoomId(1));

    assert_eq!(editor.selected_entities, HashSet::from([target]));
    assert_eq!(editor.primary_selected_entity, Some(target));
    assert_eq!(editor.inspector.selected_entity(), Some(target));
}
