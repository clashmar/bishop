// editor/src/commands/menu/add_element_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Where the element was added.
#[derive(Debug, Clone)]
enum AddTarget {
    TopLevel {
        added_index: usize,
    },
    LayoutChild {
        parent_index: usize,
        child_index: usize,
    },
}

/// Undo-able command for adding an element to a menu template.
#[derive(Debug)]
pub struct AddElementCmd {
    template_index: usize,
    element: MenuElement,
    parent_index: Option<usize>,
    target: Option<AddTarget>,
}

impl AddElementCmd {
    /// Creates a new add element command.
    /// When `parent_index` is `Some`, the element is added as a managed child of that layout group.
    pub fn new(template_index: usize, element: MenuElement, parent_index: Option<usize>) -> Self {
        Self {
            template_index,
            element,
            parent_index,
            target: None,
        }
    }
}

impl EditorCommand for AddElementCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;
            let Some(template) = menu_editor.templates.get_mut(self.template_index) else {
                return;
            };

            if let Some(parent_idx) = self.parent_index {
                if let Some(parent) = template.elements.get_mut(parent_idx) {
                    if let MenuElementKind::LayoutGroup(group) = &mut parent.kind {
                        let child = LayoutChild {
                            element: self.element.clone(),
                            managed: true,
                        };
                        group.children.push(child);
                        let child_index = group.children.len() - 1;
                        self.target = Some(AddTarget::LayoutChild {
                            parent_index: parent_idx,
                            child_index,
                        });
                    }
                }
            } else {
                template.elements.push(self.element.clone());
                let added_index = template.elements.len() - 1;
                self.target = Some(AddTarget::TopLevel { added_index });
                menu_editor.selected_element_indices.clear();
                menu_editor.selected_element_indices.insert(added_index);
                menu_editor.selected_child_index = None;
            }
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;
            let Some(template) = menu_editor.templates.get_mut(self.template_index) else {
                return;
            };

            match &self.target {
                Some(AddTarget::TopLevel { added_index }) => {
                    if *added_index < template.elements.len() {
                        template.elements.remove(*added_index);
                    }
                    menu_editor.selected_element_indices.clear();
                    menu_editor.selected_child_index = None;
                }
                Some(AddTarget::LayoutChild {
                    parent_index,
                    child_index,
                }) => {
                    if let Some(parent) = template.elements.get_mut(*parent_index) {
                        if let MenuElementKind::LayoutGroup(group) = &mut parent.kind {
                            if *child_index < group.children.len() {
                                group.children.remove(*child_index);
                            }
                        }
                    }
                }
                None => {}
            }
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Menu
    }
}
