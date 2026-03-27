// editor/src/commands/menu/reorder_child_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::menu::MenuElementKind;

/// Undo-able command for reordering a child within a layout group.
#[derive(Debug)]
pub struct ReorderChildCmd {
    template_index: usize,
    group_element_index: usize,
    from_child_index: usize,
    to_child_index: usize,
}

impl ReorderChildCmd {
    /// Creates a new reorder child command that moves a child from one position to another
    /// within the same layout group.
    pub fn new(
        template_index: usize,
        group_element_index: usize,
        from_child_index: usize,
        to_child_index: usize,
    ) -> Self {
        Self {
            template_index,
            group_element_index,
            from_child_index,
            to_child_index,
        }
    }
}

impl EditorCommand for ReorderChildCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let Some(template) = editor.menu_editor.templates.get_mut(self.template_index) else {
                return;
            };
            let Some(element) = template.elements.get_mut(self.group_element_index) else {
                return;
            };
            if let MenuElementKind::LayoutGroup(group) = &mut element.kind {
                if self.from_child_index >= group.children.len() {
                    return;
                }
                let child = group.children.remove(self.from_child_index);
                let effective = if self.to_child_index > self.from_child_index {
                    self.to_child_index - 1
                } else {
                    self.to_child_index
                };
                let insert_at = effective.min(group.children.len());
                group.children.insert(insert_at, child);
                editor.menu_editor.selected_child_index = Some(insert_at);
            }
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let Some(template) = editor.menu_editor.templates.get_mut(self.template_index) else {
                return;
            };
            let Some(element) = template.elements.get_mut(self.group_element_index) else {
                return;
            };
            if let MenuElementKind::LayoutGroup(group) = &mut element.kind {
                let effective = if self.to_child_index > self.from_child_index {
                    self.to_child_index - 1
                } else {
                    self.to_child_index
                };
                let insert_at = effective.min(group.children.len());
                if insert_at >= group.children.len() {
                    return;
                }
                let child = group.children.remove(insert_at);
                let restore_at = self.from_child_index.min(group.children.len());
                group.children.insert(restore_at, child);
                editor.menu_editor.selected_child_index = Some(restore_at);
            }
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Menu
    }
}
