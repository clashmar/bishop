use engine_core::worlds::{World, WorldId};
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct WorldSelection {
    selected: HashSet<WorldId>,
    primary: Option<WorldId>,
}

impl WorldSelection {
    pub(super) fn primary(&self) -> Option<WorldId> {
        self.primary
    }

    pub(super) fn contains(&self, world_id: WorldId) -> bool {
        self.selected.contains(&world_id)
    }

    pub(super) fn select_only(&mut self, world_id: WorldId) {
        self.selected.clear();
        self.selected.insert(world_id);
        self.primary = Some(world_id);
    }

    pub(super) fn cycle_primary(
        &mut self,
        world_order: &[World],
        forward: bool,
    ) -> Option<WorldId> {
        if world_order.is_empty() {
            return None;
        }

        let current_index = self
            .primary
            .and_then(|primary| world_order.iter().position(|world| world.id == primary));
        let target_index = match current_index {
            Some(index) if forward => (index + 1) % world_order.len(),
            Some(index) => (index + world_order.len() - 1) % world_order.len(),
            None if forward => 0,
            None => world_order.len() - 1,
        };
        let target = world_order[target_index].id;
        self.select_only(target);
        Some(target)
    }

    pub(super) fn toggle(&mut self, world_id: WorldId) {
        if self.selected.remove(&world_id) {
            if self.primary == Some(world_id) {
                self.primary = self.selected.iter().copied().min();
            }
            return;
        }

        self.selected.insert(world_id);
        self.primary = Some(world_id);
    }

    pub(super) fn add_all_with_minimum_primary(&mut self, world_ids: &[WorldId]) -> bool {
        let Some(primary) = world_ids.iter().copied().min() else {
            return false;
        };
        self.selected.extend(world_ids.iter().copied());
        self.primary = Some(primary);
        true
    }

    pub(super) fn set_primary(&mut self, world_id: WorldId) {
        if self.selected.contains(&world_id) {
            self.primary = Some(world_id);
        }
    }

    pub(super) fn clear(&mut self) {
        self.selected.clear();
        self.primary = None;
    }

    pub(super) fn retain_existing_ids(&mut self, existing_ids: &HashSet<WorldId>) {
        self.selected.retain(|world_id| existing_ids.contains(world_id));
        if self.primary.is_some_and(|world_id| !self.selected.contains(&world_id)) {
            self.primary = self.selected.iter().copied().min();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WorldSelection;
    use engine_core::worlds::{World, WorldId};
    use std::collections::HashSet;

    #[test]
    fn world_selection_plain_click_replaces_selection() {
        let mut selection = WorldSelection::default();
        let first = WorldId(1);
        let second = WorldId(2);

        selection.select_only(first);
        selection.select_only(second);

        assert_eq!(selection.primary(), Some(second));
        assert!(!selection.contains(first));
        assert!(selection.contains(second));
    }

    #[test]
    fn world_selection_shift_click_toggles_membership() {
        let mut selection = WorldSelection::default();
        let first = WorldId(1);
        let second = WorldId(2);

        selection.select_only(first);
        selection.toggle(second);
        assert!(selection.contains(second));
        assert_eq!(selection.primary(), Some(second));

        selection.toggle(second);
        assert!(!selection.contains(second));
        assert_eq!(selection.primary(), Some(first));
    }

    #[test]
    fn world_selection_toggling_primary_chooses_fallback() {
        let mut selection = WorldSelection::default();
        let first = WorldId(1);
        let second = WorldId(2);
        let third = WorldId(3);

        selection.select_only(third);
        selection.toggle(second);
        selection.toggle(first);
        selection.toggle(first);

        assert_eq!(selection.primary(), Some(second));
    }

    #[test]
    fn world_selection_clear_removes_primary_and_members() {
        let mut selection = WorldSelection::default();
        selection.select_only(WorldId(1));

        selection.clear();

        assert_eq!(selection.primary(), None);
        assert!(!selection.contains(WorldId(1)));
    }

    #[test]
    fn world_selection_retain_existing_ids_clears_stale_primary() {
        let mut selection = WorldSelection::default();
        let existing = WorldId(1);
        let removed = WorldId(2);
        selection.select_only(existing);
        selection.toggle(removed);

        selection.retain_existing_ids(&HashSet::from([existing]));

        assert_eq!(selection.primary(), Some(existing));
        assert!(!selection.contains(removed));
    }

    #[test]
    fn world_cycle_next_follows_game_order_and_replaces_selection() {
        let worlds = world_order();
        let mut selection = WorldSelection::default();
        selection.select_only(WorldId(1));
        selection.toggle(WorldId(2));

        assert_eq!(selection.cycle_primary(&worlds, true), Some(WorldId(3)));
        assert!(!selection.contains(WorldId(1)));
        assert!(!selection.contains(WorldId(2)));
        assert!(selection.contains(WorldId(3)));
        assert_eq!(selection.primary(), Some(WorldId(3)));
    }

    #[test]
    fn world_cycle_previous_wraps_to_last_world() {
        let worlds = world_order();
        let mut selection = WorldSelection::default();
        selection.select_only(WorldId(1));

        assert_eq!(selection.cycle_primary(&worlds, false), Some(WorldId(3)));
        assert_eq!(selection.primary(), Some(WorldId(3)));
    }

    #[test]
    fn world_cycle_next_wraps_to_first_world() {
        let worlds = world_order();
        let mut selection = WorldSelection::default();
        selection.select_only(WorldId(3));

        assert_eq!(selection.cycle_primary(&worlds, true), Some(WorldId(1)));
    }

    #[test]
    fn world_cycle_without_selection_chooses_order_boundary() {
        let worlds = world_order();
        let mut selection = WorldSelection::default();

        assert_eq!(selection.cycle_primary(&worlds, true), Some(WorldId(1)));
        assert_eq!(selection.cycle_primary(&worlds, false), Some(WorldId(3)));
    }

    #[test]
    fn world_cycle_with_no_worlds_does_nothing() {
        let mut selection = WorldSelection::default();

        assert_eq!(selection.cycle_primary(&[], true), None);
        assert_eq!(selection.primary(), None);
    }

    fn world_order() -> [World; 3] {
        [
            World::new(WorldId(1), "First".to_owned(), 16.0),
            World::new(WorldId(2), "Second".to_owned(), 16.0),
            World::new(WorldId(3), "Third".to_owned(), 16.0),
        ]
    }

    #[test]
    fn world_selection_additive_selection_preserves_existing_members() {
        let mut selection = WorldSelection::default();
        let first = WorldId(1);
        let second = WorldId(2);
        selection.select_only(first);

        selection.toggle(second);

        assert!(selection.contains(first));
        assert!(selection.contains(second));
        assert_eq!(selection.primary(), Some(second));
    }
}
