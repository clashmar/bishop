use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for moving a top-level element into a layout group.
#[derive(Debug)]
pub struct MoveElementInCmd {
    template_index: usize,
    element_index: usize,
    group_index: usize,
    insert_child_index: usize,
    original_element: Option<MenuElement>,
}

impl MoveElementInCmd {
    pub fn new(
        template_index: usize,
        element_index: usize,
        group_index: usize,
        insert_child_index: usize,
    ) -> Self {
        Self {
            template_index,
            element_index,
            group_index,
            insert_child_index,
            original_element: None,
        }
    }
}

impl EditorCommand for MoveElementInCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;
            let Some(template) = menu_editor.templates.get_mut(self.template_index) else {
                return;
            };
            if self.element_index >= template.elements.len() || self.group_index >= template.elements.len() {
                return;
            }

            self.original_element = Some(template.elements[self.element_index].clone());
            let removed = template.elements.remove(self.element_index);
            let adjusted_group_index = if self.group_index > self.element_index {
                self.group_index - 1
            } else {
                self.group_index
            };

            let group_rect = template.elements[adjusted_group_index].rect;
            let mut element = removed;
            element.rect.x -= group_rect.x;
            element.rect.y -= group_rect.y;

            let insert_at = match &template.elements[adjusted_group_index].kind {
                MenuElementKind::LayoutGroup(g) => self.insert_child_index.min(g.children.len()),
                _ => return,
            };

            if let MenuElementKind::LayoutGroup(group) = &mut template.elements[adjusted_group_index].kind {
                group.children.insert(
                    insert_at,
                    LayoutChild {
                        element,
                        managed: true,
                    },
                );
            }

            self.group_index = adjusted_group_index;

            menu_editor.selected_element_indices.clear();
            menu_editor.selected_element_indices.insert(adjusted_group_index);
            menu_editor.selected_child_index = Some(insert_at);
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;
            let Some(template) = menu_editor.templates.get_mut(self.template_index) else {
                return;
            };
            let Some(original) = self.original_element.take() else {
                return;
            };
            if self.group_index >= template.elements.len() {
                return;
            }

            if let MenuElementKind::LayoutGroup(group) = &mut template.elements[self.group_index].kind {
                if self.insert_child_index < group.children.len() {
                    group.children.remove(self.insert_child_index);
                }
            }

            let insert_at = self.element_index.min(template.elements.len());
            template.elements.insert(insert_at, original);
            menu_editor.selected_element_indices.clear();
            menu_editor.selected_element_indices.insert(insert_at);
            menu_editor.selected_child_index = None;
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Menu
    }
}
