use super::MenuManager;
use crate::menu::{MenuElement, MenuElementKind, MenuEnabledWhen, MenuTemplate};

impl MenuManager {
    /// Sets whether a committed latest save satisfies menu availability conditions.
    pub fn set_latest_save_available(&mut self, available: bool) {
        if self.latest_save_available == available {
            return;
        }
        self.latest_save_available = available;
        if !available
            && let Some(menu_id) = self.menu_stack.last()
            && let Some(template) = self.displayed_template(menu_id)
        {
            self.focus.reset(&template);
        }
    }

    pub(super) fn displayed_template(&self, id: &str) -> Option<MenuTemplate> {
        let mut template = self.templates.get(id)?.clone();
        for element in &mut template.elements {
            apply_availability(element, self.latest_save_available, true);
        }
        Some(template)
    }
}

fn apply_availability(element: &mut MenuElement, latest_save_available: bool, parent_enabled: bool) {
    element.enabled &= parent_enabled
        && (element.enabled_when != MenuEnabledWhen::HasLatestSave || latest_save_available);
    if let MenuElementKind::LayoutGroup(group) = &mut element.kind {
        for child in &mut group.children {
            apply_availability(&mut child.element, latest_save_available, element.enabled);
        }
    }
}
