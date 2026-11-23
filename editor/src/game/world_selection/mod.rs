use engine_core::worlds::WorldId;
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
    use engine_core::worlds::WorldId;
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
