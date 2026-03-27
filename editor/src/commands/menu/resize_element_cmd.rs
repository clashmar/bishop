// editor/src/commands/menu/resize_element_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use bishop::prelude::*;
use engine_core::prelude::*;

/// Undo-able command for resizing a menu element.
#[derive(Debug)]
pub struct ResizeElementCmd {
    template_index: usize,
    element_index: usize,
    child_index: Option<usize>,
    old_rect: Rect,
    new_rect: Rect,
}

impl ResizeElementCmd {
    pub fn new(
        template_index: usize,
        element_index: usize,
        child_index: Option<usize>,
        old_rect: Rect,
        new_rect: Rect,
    ) -> Self {
        Self {
            template_index,
            element_index,
            child_index,
            old_rect,
            new_rect,
        }
    }
}

impl ResizeElementCmd {
    fn apply_rect(&self, rect: Rect) {
        with_editor(|editor| {
            let Some(template) = editor.menu_editor.templates.get_mut(self.template_index) else {
                return;
            };
            if let Some(child_idx) = self.child_index {
                if let Some(element) = template.elements.get_mut(self.element_index) {
                    if let MenuElementKind::LayoutGroup(group) = &mut element.kind {
                        if let Some(child) = group.children.get_mut(child_idx) {
                            child.element.rect = rect;
                        }
                    }
                }
            } else if let Some(element) = template.elements.get_mut(self.element_index) {
                element.rect = rect;
            }
        });
    }
}

impl EditorCommand for ResizeElementCmd {
    fn execute(&mut self) {
        self.apply_rect(self.new_rect);
    }

    fn undo(&mut self) {
        self.apply_rect(self.old_rect);
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Menu
    }
}
