// editor/src/commands/room/set_parent_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::ecs::entity::*;

/// Undo-able command for setting an entity's parent.
#[derive(Debug)]
pub struct SetParentCmd {
    child: Entity,
    new_parent: Entity,
    old_parent: Option<Entity>,
    mode: EditorMode,
}

impl SetParentCmd {
    pub fn new(
        child: Entity,
        new_parent: Entity,
        old_parent: Option<Entity>,
        mode: EditorMode,
    ) -> Self {
        Self {
            child,
            new_parent,
            old_parent,
            mode,
        }
    }
}

impl EditorCommand for SetParentCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let ecs = &mut editor.game.ecs;
            set_parent(ecs, self.child, self.new_parent);
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let ecs = &mut editor.game.ecs;
            if let Some(old_parent) = self.old_parent {
                set_parent(ecs, self.child, old_parent);
            } else {
                remove_parent(ecs, self.child);
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
