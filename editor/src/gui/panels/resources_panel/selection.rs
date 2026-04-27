use super::{context_menu::*, ResourcesPanel};
use bishop::prelude::*;
use std::path::PathBuf;

impl ResourcesPanel {
    pub(crate) fn clear_selection(&mut self) {
        self.selected_index = None;
    }

    pub(crate) fn handle_primary_click_on_entry(
        &mut self,
        entry_index: usize,
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
            self.selected_index = Some(entry_index);
            return None;
        }

        if is_dir_like {
            self.clear_selection();
            if is_parent {
                self.navigation.pop();
            } else {
                self.navigation.push(&name);
            }
            return None;
        }

        self.selected_index = Some(entry_index);
        Some(path)
    }

    pub(crate) fn handle_secondary_click_on_entry(&mut self, entry_index: usize, position: Vec2) {
        let Some(entry) = self.entries.get(entry_index) else {
            return;
        };

        self.selected_index = Some(entry_index);
        self.active_menu =
            context_target_for_entry(entry_index, entry, position).map(ActiveMenu::Entry);
    }

    pub(crate) fn handle_secondary_click_on_background(&mut self, position: Vec2) {
        self.clear_selection();
        self.active_menu = Some(context_target_for_background(position));
    }
}
