use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for moving a layout-group child to the top level.
#[derive(Debug)]
pub struct MoveElementOutCmd {
    template_index: usize,
    group_index: usize,
    child_index: usize,
    insert_element_index: usize,
    original_child: Option<LayoutChild>,
    resolved_abs_rect: Rect,
}

impl MoveElementOutCmd {
    pub fn new(
        template_index: usize,
        group_index: usize,
        child_index: usize,
        insert_element_index: usize,
        resolved_abs_rect: Rect,
    ) -> Self {
        Self {
            template_index,
            group_index,
            child_index,
            insert_element_index,
            original_child: None,
            resolved_abs_rect,
        }
    }
}

impl EditorCommand for MoveElementOutCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;
            let Some(template) = menu_editor.templates.get_mut(self.template_index) else {
                return;
            };
            if self.group_index >= template.elements.len() {
                return;
            }

            let child = if let MenuElementKind::LayoutGroup(group) = &mut template.elements[self.group_index].kind {
                if self.child_index >= group.children.len() {
                    return;
                }
                group.children.remove(self.child_index)
            } else {
                return;
            };

            self.original_child = Some(child.clone());
            let mut element = child.element;
            element.rect = self.resolved_abs_rect;

            let insert_at = self.insert_element_index.min(template.elements.len());
            template.elements.insert(insert_at, element);

            menu_editor.selected_element_indices.clear();
            menu_editor.selected_element_indices.insert(insert_at);
            menu_editor.selected_child_index = None;
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;
            let Some(template) = menu_editor.templates.get_mut(self.template_index) else {
                return;
            };
            let Some(original_child) = self.original_child.take() else {
                return;
            };
            if self.group_index >= template.elements.len() {
                return;
            }

            let remove_at = self.insert_element_index.min(template.elements.len().saturating_sub(1));
            if remove_at < template.elements.len() {
                template.elements.remove(remove_at);
            }

            if let MenuElementKind::LayoutGroup(group) = &mut template.elements[self.group_index].kind {
                let restore_at = self.child_index.min(group.children.len());
                group.children.insert(restore_at, original_child);
            }

            menu_editor.selected_element_indices.clear();
            menu_editor.selected_element_indices.insert(self.group_index);
            menu_editor.selected_child_index = Some(self.child_index);
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Menu
    }
}
