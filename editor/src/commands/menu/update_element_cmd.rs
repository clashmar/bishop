// editor/src/commands/menu/update_element_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for updating any property on a menu element.
/// Stores full before/after clones of the element.
#[derive(Debug)]
pub struct UpdateElementCmd {
    template_index: usize,
    element_index: usize,
    child_index: Option<usize>,
    old_element: MenuElement,
    new_element: MenuElement,
}

impl UpdateElementCmd {
    pub fn new(
        template_index: usize,
        element_index: usize,
        child_index: Option<usize>,
        old_element: MenuElement,
        new_element: MenuElement,
    ) -> Self {
        Self {
            template_index,
            element_index,
            child_index,
            old_element,
            new_element,
        }
    }
}

impl UpdateElementCmd {
    fn apply_element(&self, element: &MenuElement) {
        with_editor(|editor| {
            let Some(template) = editor.menu_editor.templates.get_mut(self.template_index) else {
                return;
            };
            if let Some(child_idx) = self.child_index {
                if let Some(parent) = template.elements.get_mut(self.element_index) {
                    if let MenuElementKind::LayoutGroup(group) = &mut parent.kind {
                        if let Some(child) = group.children.get_mut(child_idx) {
                            child.element = element.clone();
                        }
                    }
                }
            } else if let Some(target) = template.elements.get_mut(self.element_index) {
                *target = element.clone();
            }
        });
    }
}

impl EditorCommand for UpdateElementCmd {
    fn execute(&mut self) {
        self.apply_element(&self.new_element.clone());
    }

    fn undo(&mut self) {
        self.apply_element(&self.old_element.clone());
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Menu
    }
}
