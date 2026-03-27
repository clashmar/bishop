// editor/src/commands/menu/create_template_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for creating a new menu template.
#[derive(Debug)]
pub struct CreateTemplateCmd {
    id: String,
    created_index: Option<usize>,
    saved_template: Option<MenuTemplate>,
}

impl CreateTemplateCmd {
    pub fn new(id: String) -> Self {
        Self {
            id,
            created_index: None,
            saved_template: None,
        }
    }
}

impl EditorCommand for CreateTemplateCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;

            let template = self
                .saved_template
                .take()
                .unwrap_or_else(|| MenuTemplate::new(self.id.clone()));
            menu_editor.templates.push(template);

            let index = menu_editor.templates.len() - 1;
            self.created_index = Some(index);
            menu_editor.current_template_index = Some(index);
            menu_editor.selected_element_indices.clear();
            menu_editor.selected_child_index = None;
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;

            if let Some(index) = self.created_index.take() {
                if index < menu_editor.templates.len() {
                    self.saved_template = Some(menu_editor.templates.remove(index));
                }

                if menu_editor.templates.is_empty() {
                    menu_editor.current_template_index = None;
                } else if let Some(current) = menu_editor.current_template_index {
                    if current >= menu_editor.templates.len() {
                        menu_editor.current_template_index = Some(menu_editor.templates.len() - 1);
                    }
                }
                menu_editor.selected_element_indices.clear();
                menu_editor.selected_child_index = None;
            }
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Menu
    }
}
