// editor/src/commands/menu/move_element_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use bishop::prelude::*;
use engine_core::prelude::*;

/// A single element move with from/to positions.
#[derive(Debug, Clone)]
pub struct ElementMove {
    pub element_index: usize,
    pub child_index: Option<usize>,
    pub from: Vec2,
    pub to: Vec2,
}

/// Undo-able command for moving one or more menu elements.
#[derive(Debug)]
pub struct MoveElementCmd {
    template_index: usize,
    moves: Vec<ElementMove>,
}

impl MoveElementCmd {
    pub fn new(template_index: usize, moves: Vec<ElementMove>) -> Self {
        Self {
            template_index,
            moves,
        }
    }
}

impl MoveElementCmd {
    fn apply_positions(&self, use_to: bool) {
        with_editor(|editor| {
            let Some(template) = editor.menu_editor.templates.get_mut(self.template_index) else {
                return;
            };
            for m in &self.moves {
                let pos = if use_to { m.to } else { m.from };
                if let Some(child_idx) = m.child_index {
                    if let Some(element) = template.elements.get_mut(m.element_index) {
                        if let MenuElementKind::LayoutGroup(group) = &mut element.kind {
                            if let Some(child) = group.children.get_mut(child_idx) {
                                child.element.rect.x = pos.x;
                                child.element.rect.y = pos.y;
                            }
                        }
                    }
                } else if let Some(element) = template.elements.get_mut(m.element_index) {
                    element.rect.x = pos.x;
                    element.rect.y = pos.y;
                }
            }
        });
    }
}

impl EditorCommand for MoveElementCmd {
    fn execute(&mut self) {
        self.apply_positions(true);
    }

    fn undo(&mut self) {
        self.apply_positions(false);
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Menu
    }
}
