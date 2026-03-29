// editor/src/commands/room/remove_parent_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::ecs::entity::*;

/// Undo-able command for removing an entity's parent.
#[derive(Debug)]
pub struct RemoveParentCmd {
    child: Entity,
    old_parent: Option<Entity>,
    mode: EditorMode,
}

impl RemoveParentCmd {
    pub fn new(child: Entity, old_parent: Option<Entity>, mode: EditorMode) -> Self {
        Self {
            child,
            old_parent,
            mode,
        }
    }
}

impl EditorCommand for RemoveParentCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let ecs = &mut editor.game.ecs;
            remove_parent(ecs, self.child);
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let ecs = &mut editor.game.ecs;
            if let Some(old_parent) = self.old_parent {
                set_parent(ecs, self.child, old_parent);
            }
        });
    }

    fn mode(&self) -> EditorMode {
        self.mode
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        match self.mode {
            // Global entities can be undone from Game mode or any Room mode
            EditorMode::Game => matches!(current_mode, EditorMode::Game | EditorMode::Room(_)),
            other => other == current_mode,
        }
    }
}
