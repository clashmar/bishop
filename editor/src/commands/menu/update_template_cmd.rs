// editor/src/commands/menu/update_template_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Which template property changed.
#[derive(Debug, Clone)]
pub enum TemplateProperty {
    Name {
        old: String,
        new: String,
    },
    Mode {
        old: MenuMode,
        new: MenuMode,
    },
    Background {
        old: MenuBackground,
        new: MenuBackground,
    },
}

/// Undo-able command for changing a menu template property.
#[derive(Debug)]
pub struct UpdateTemplateCmd {
    template_index: usize,
    property: TemplateProperty,
}

impl UpdateTemplateCmd {
    pub fn new(template_index: usize, property: TemplateProperty) -> Self {
        Self {
            template_index,
            property,
        }
    }
}

impl UpdateTemplateCmd {
    fn apply(&self, editor: &mut crate::app::Editor, use_new: bool) {
        let Some(template) = editor.menu_editor.templates.get_mut(self.template_index) else {
            return;
        };
        match &self.property {
            TemplateProperty::Name { old, new } => {
                template.id = if use_new { new } else { old }.clone();
            }
            TemplateProperty::Mode { old, new } => {
                template.mode = if use_new { *new } else { *old };
            }
            TemplateProperty::Background { old, new } => {
                template.background = if use_new { *new } else { *old };
            }
        }
    }
}

impl EditorCommand for UpdateTemplateCmd {
    fn execute(&mut self) {
        with_editor(|editor| self.apply(editor, true));
    }

    fn undo(&mut self) {
        with_editor(|editor| self.apply(editor, false));
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Menu
    }
}
