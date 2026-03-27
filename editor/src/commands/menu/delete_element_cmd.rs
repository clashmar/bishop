// editor/src/commands/menu/delete_element_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;
use std::collections::HashSet;

/// Tracks what was deleted for restoration on undo.
#[derive(Debug, Clone)]
enum DeletedEntry {
    TopLevel {
        index: usize,
        element: MenuElement,
    },
    LayoutChild {
        parent_index: usize,
        child_index: usize,
        child: LayoutChild,
    },
}

/// Undo-able command for deleting selected menu element(s).
#[derive(Debug)]
pub struct DeleteElementCmd {
    template_index: usize,
    selected_indices: HashSet<usize>,
    selected_child_index: Option<usize>,
    deleted: Vec<DeletedEntry>,
    previous_selection: HashSet<usize>,
    previous_child_index: Option<usize>,
}

impl DeleteElementCmd {
    /// Creates a delete command from the current selection state.
    pub fn new(
        template_index: usize,
        selected_indices: HashSet<usize>,
        selected_child_index: Option<usize>,
    ) -> Self {
        Self {
            template_index,
            selected_indices: selected_indices.clone(),
            selected_child_index,
            deleted: Vec::new(),
            previous_selection: selected_indices,
            previous_child_index: selected_child_index,
        }
    }
}

impl EditorCommand for DeleteElementCmd {
    fn execute(&mut self) {
        self.deleted.clear();

        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;
            let Some(template) = menu_editor.templates.get_mut(self.template_index) else {
                return;
            };

            // Single parent with child selected: delete the child only
            if self.selected_indices.len() == 1 {
                if let Some(child_idx) = self.selected_child_index {
                    let parent_idx = *self.selected_indices.iter().next().unwrap();
                    if let Some(element) = template.elements.get_mut(parent_idx) {
                        if let MenuElementKind::LayoutGroup(group) = &mut element.kind {
                            if child_idx < group.children.len() {
                                let child = group.children.remove(child_idx);
                                self.deleted.push(DeletedEntry::LayoutChild {
                                    parent_index: parent_idx,
                                    child_index: child_idx,
                                    child,
                                });

                                menu_editor.selected_child_index = if group.children.is_empty() {
                                    None
                                } else if child_idx >= group.children.len() {
                                    Some(group.children.len() - 1)
                                } else {
                                    Some(child_idx)
                                };
                            }
                        }
                    }
                    return;
                }
            }

            // Multi-delete top-level: sort descending and remove
            let mut indices: Vec<usize> = self.selected_indices.iter().copied().collect();
            indices.sort_unstable_by(|a, b| b.cmp(a));

            for index in indices {
                if index < template.elements.len() {
                    let element = template.elements.remove(index);
                    self.deleted.push(DeletedEntry::TopLevel { index, element });
                }
            }

            menu_editor.selected_element_indices.clear();
            menu_editor.selected_child_index = None;
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;
            let Some(template) = menu_editor.templates.get_mut(self.template_index) else {
                return;
            };

            // Re-insert in reverse order (deleted were stored in removal order)
            for entry in self.deleted.iter().rev() {
                match entry {
                    DeletedEntry::TopLevel { index, element } => {
                        let insert_at = (*index).min(template.elements.len());
                        template.elements.insert(insert_at, element.clone());
                    }
                    DeletedEntry::LayoutChild {
                        parent_index,
                        child_index,
                        child,
                    } => {
                        if let Some(parent) = template.elements.get_mut(*parent_index) {
                            if let MenuElementKind::LayoutGroup(group) = &mut parent.kind {
                                let insert_at = (*child_index).min(group.children.len());
                                group.children.insert(insert_at, child.clone());
                            }
                        }
                    }
                }
            }

            menu_editor.selected_element_indices = self.previous_selection.clone();
            menu_editor.selected_child_index = self.previous_child_index;
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Menu
    }
}
