use crate::gui::panels::hierarchy_panel::HIERARCHY_PANEL;
use bishop::prelude::Vec2;
use engine_core::ecs::{Ecs, Entity, Transform};
use std::collections::HashSet;
use widgets::{focused_panel, input_is_focused, is_context_menu_open, is_modal_open};

pub(super) fn room_movement_shortcuts_blocked() -> bool {
    input_is_focused()
        || is_modal_open()
        || is_context_menu_open()
        || focused_panel().is_some_and(|panel| panel != HIERARCHY_PANEL)
}

pub(super) fn move_selected_entities(
    ecs: &mut Ecs,
    selected_entities: &HashSet<Entity>,
    step: Vec2,
) -> Vec<(Entity, Vec2, Vec2)> {
    let mut moves = Vec::new();
    for &entity in selected_entities {
        if let Some(transform) = ecs.get_store_mut::<Transform>().get_mut(entity) {
            let old = transform.position;
            transform.position += step;
            moves.push((entity, old, transform.position));
        }
    }
    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use bishop::prelude::vec2;
    use engine_core::ecs::{Ecs, Parent, Transform};
    use std::collections::HashSet;
    use widgets::{clear_all_input_focus, set_context_menu_open, set_focused_panel, set_modal_open};

    #[test]
    fn hierarchy_panel_focus_does_not_block_room_keyboard_movement() {
        clear_all_input_focus();
        set_context_menu_open(false);
        set_modal_open(false);
        set_focused_panel(Some(HIERARCHY_PANEL));

        assert!(!room_movement_shortcuts_blocked());

        set_focused_panel(None);
    }

    #[test]
    fn other_panel_focus_still_blocks_room_keyboard_movement() {
        clear_all_input_focus();
        set_context_menu_open(false);
        set_modal_open(false);
        set_focused_panel(Some(crate::gui::panels::console_panel::CONSOLE_PANEL));

        assert!(room_movement_shortcuts_blocked());

        set_focused_panel(None);
    }

    #[test]
    fn focused_text_input_blocks_room_keyboard_movement_with_hierarchy_focused() {
        set_focused_panel(Some(HIERARCHY_PANEL));
        widgets::INPUT_FOCUSED.with(|focused| *focused.borrow_mut() = true);

        assert!(room_movement_shortcuts_blocked());

        clear_all_input_focus();
        set_focused_panel(None);
    }

    #[test]
    fn keyboard_movement_for_selected_child_without_room_layer_membership_moves_transform() {
        let mut ecs = Ecs::default();
        let parent = ecs.create_entity().with(Transform::default()).finish();
        let child = ecs
            .create_entity()
            .with(Transform {
                position: vec2(4.0, 5.0),
                ..Default::default()
            })
            .with(Parent(parent))
            .finish();
        let selected = HashSet::from([child]);

        let moves = move_selected_entities(&mut ecs, &selected, vec2(1.0, 0.0));

        assert_eq!(moves, vec![(child, vec2(4.0, 5.0), vec2(5.0, 5.0))]);
        assert_eq!(
            ecs.get::<Transform>(child)
                .map(|transform| transform.position),
            Some(vec2(5.0, 5.0))
        );
    }
}
