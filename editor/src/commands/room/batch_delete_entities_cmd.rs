// editor/src/commands/room/batch_delete_entities_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for deleting multiple entities and their children.
#[derive(Debug)]
pub struct BatchDeleteEntitiesCmd {
    pub entities: Vec<Entity>,
    pub mode: EditorMode,
    pub saved: Option<GroupSnapshot>,
}

impl BatchDeleteEntitiesCmd {
    pub fn new(entities: Vec<Entity>, mode: EditorMode) -> Self {
        Self {
            entities,
            mode,
            saved: None,
        }
    }
}

impl EditorCommand for BatchDeleteEntitiesCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let ctx = &mut editor.game.ctx_mut();

            // Capture all entities before deleting
            let mut all_saved = Vec::new();
            for &entity in &self.entities {
                let captured = capture_subtree(ctx.ecs, entity);
                all_saved.extend(captured);
            }
            self.saved = Some(all_saved);

            // Delete all entities
            for &entity in &self.entities {
                Ecs::remove_entity(ctx, entity);
            }

            editor.room_editor.clear_selection();
        });
    }

    fn undo(&mut self) {
        if let Some(saved) = self.saved.take() {
            with_editor(|editor| {
                let ctx = &mut editor.game.ctx_mut();
                restore_subtree(ctx, &saved);

                // Re-select the entities
                for &entity in &self.entities {
                    editor.room_editor.add_to_selection(entity);
                }
            });
        }
    }

    fn mode(&self) -> EditorMode {
        self.mode
    }
}
