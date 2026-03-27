// editor/src/commands/menu/delete_template_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::storage::editor_storage::delete_menu;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for deleting a menu template.
#[derive(Debug)]
pub struct DeleteTemplateCmd {
    template_index: usize,
    saved_template: Option<MenuTemplate>,
    previous_template_index: Option<usize>,
}

impl DeleteTemplateCmd {
    pub fn new(template_index: usize) -> Self {
        Self {
            template_index,
            saved_template: None,
            previous_template_index: None,
        }
    }
}

impl EditorCommand for DeleteTemplateCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;

            if self.template_index >= menu_editor.templates.len() {
                return;
            }

            self.previous_template_index = menu_editor.current_template_index;

            let template = menu_editor.templates.remove(self.template_index);
            if let Err(err) = delete_menu(&template.id) {
                onscreen_error!("Error deleting template file: {err}");
            }
            self.saved_template = Some(template);

            if menu_editor.templates.is_empty() {
                menu_editor.current_template_index = None;
            } else if let Some(current) = menu_editor.current_template_index {
                if current >= menu_editor.templates.len() {
                    menu_editor.current_template_index = Some(menu_editor.templates.len() - 1);
                } else if current > self.template_index {
                    menu_editor.current_template_index = Some(current - 1);
                }
            }
            menu_editor.selected_element_indices.clear();
            menu_editor.selected_child_index = None;
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let menu_editor = &mut editor.menu_editor;

            if let Some(template) = self.saved_template.take() {
                let index = self.template_index.min(menu_editor.templates.len());
                menu_editor.templates.insert(index, template);
                menu_editor.current_template_index = self.previous_template_index;
                menu_editor.selected_element_indices.clear();
                menu_editor.selected_child_index = None;
            }
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Menu
    }
}
