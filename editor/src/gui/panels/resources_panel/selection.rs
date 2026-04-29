use std::collections::BTreeSet;

use super::{context_menu::*, MarqueeSelectionState, ResourcesPanel};
use bishop::prelude::*;
use engine_core::prelude::*;
use std::path::PathBuf;

impl ResourcesPanel {
    /// Deselects all entries.
    pub(crate) fn clear_selection(&mut self) {
        self.selected_indices.clear();
    }

    /// Replaces the current selection with a single entry.
    pub(crate) fn set_single_selection(&mut self, entry_index: usize) {
        self.selected_indices.clear();
        self.selected_indices.insert(entry_index);
    }

    /// Toggles an entry in or out of the current selection.
    pub(crate) fn toggle_selection(&mut self, entry_index: usize) {
        if !self.selected_indices.insert(entry_index) {
            self.selected_indices.remove(&entry_index);
        }
    }

    /// Handles a primary click on a resource entry.
    pub(crate) fn handle_primary_click_on_entry(
        &mut self,
        entry_index: usize,
        shift_held: bool,
        double_clicked: bool,
    ) -> Option<PathBuf> {
        let (is_dir_like, is_parent, name, path) = {
            let entry = self.entries.get(entry_index)?;
            (
                entry.is_dir_like(),
                entry.is_parent(),
                entry.name.clone(),
                entry.path.clone(),
            )
        };

        if !double_clicked {
            if is_parent {
                return None;
            }
            if shift_held {
                self.toggle_selection(entry_index);
            } else {
                self.set_single_selection(entry_index);
            }
            return None;
        }

        if is_dir_like {
            self.clear_selection();
            self.reset_marquee_selection();
            if is_parent {
                self.navigation.pop();
            } else {
                self.navigation.push(&name);
            }
            return None;
        }

        self.set_single_selection(entry_index);
        Some(path)
    }

    /// Begins a marquee drag-selection, snapshotting the current selection.
    pub(crate) fn begin_marquee_selection(&mut self, start_content_pos: Vec2, additive: bool) {
        self.marquee_selection.active = true;
        self.marquee_selection.additive = additive;
        self.marquee_selection.start_content_pos = Some(start_content_pos);
        self.marquee_selection.selection_snapshot = self.selected_indices.clone();
        if !additive {
            self.selected_indices.clear();
        }
    }

    /// Commits marquee selection. For additive drags, entries in the
    /// snapshot that are also matched get deselected, while matched entries not
    /// in the snapshot get selected.
    pub(crate) fn commit_marquee_selection(&mut self, matched_indices: BTreeSet<usize>) {
        if self.marquee_selection.additive {
            let snapshot = std::mem::take(&mut self.marquee_selection.selection_snapshot);
            self.selected_indices = snapshot.clone();
            for index in &matched_indices {
                if snapshot.contains(index) {
                    self.selected_indices.remove(index);
                } else {
                    self.selected_indices.insert(*index);
                }
            }
        } else {
            self.selected_indices = matched_indices;
        }
        self.reset_marquee_selection();
    }

    /// Resets marquee drag-selection state.
    pub(crate) fn reset_marquee_selection(&mut self) {
        self.marquee_selection = MarqueeSelectionState::default();
    }

    pub(crate) fn handle_secondary_click_on_entry(&mut self, entry_index: usize, position: Vec2) {
        if self.selected_indices.len() > 1 && self.selected_indices.contains(&entry_index) {
            let all_deletable = self.selected_indices.iter().all(|&i| {
                self.entries.get(i).is_some_and(|e| {
                    !matches!(e.kind, EntryKind::Parent | EntryKind::SystemDirectory)
                })
            });
            if all_deletable {
                self.reset_marquee_selection();
                self.active_menu = Some(ActiveMenu::MultiSelection(position));
                return;
            }
        }

        let menu = self.entries.get(entry_index).and_then(|entry| {
            context_target_for_entry(entry_index, entry, position).map(ActiveMenu::Entry)
        });

        if let Some(menu) = menu {
            self.set_single_selection(entry_index);
            self.reset_marquee_selection();
            self.active_menu = Some(menu);
        }
    }

    pub(crate) fn handle_secondary_click_on_background(&mut self, position: Vec2) {
        self.clear_selection();
        self.active_menu = Some(context_target_for_background(position));
    }

    /// Returns a pending delete action for the current selection.
    pub(crate) fn pending_delete_for_selection(
        &self,
        registry: &AssetRegistry,
    ) -> Option<PendingResourceAction> {
        if self.selected_indices.is_empty() {
            return None;
        }
        if self.selected_indices.len() == 1 {
            let &index = self.selected_indices.iter().next()?;
            let entry = self.entries.get(index)?;
            return pending_action_for(entry, ResourceMenuAction::Delete, registry);
        }

        let mut targets = Vec::new();
        for &index in &self.selected_indices {
            let entry = self.entries.get(index)?;
            if matches!(entry.kind, EntryKind::Parent | EntryKind::SystemDirectory) {
                return None;
            }
            let target = delete_target_for(entry, registry)?;
            targets.push(target);
        }
        Some(PendingResourceAction::BatchDelete(targets))
    }
}
