// editor/src/commands/room/delete_entity_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;
/// Undo-able command for deleting an entity and its children.
#[derive(Debug)]
pub struct DeleteEntityCmd {
    pub entity: Entity,
    pub room_id: RoomId,
    pub saved: Option<GroupSnapshot>,
}

impl EditorCommand for DeleteEntityCmd {
    fn execute(&mut self) {
        // Capture components before deleting
        with_editor(|editor| {
            let ctx = &mut editor.game.ctx_mut();
            self.saved = Some(capture_subtree(ctx.ecs, self.entity));
            Ecs::remove_entity(ctx, self.entity);
            editor.room_editor.set_selected_entity(None);
        });
    }

    fn undo(&mut self) {
        if let Some(saved) = self.saved.take() {
            with_editor(|editor| {
                let ctx = &mut editor.game.ctx_mut();
                // Restore every entity and its components
                restore_subtree(ctx, &saved);
                editor.room_editor.set_selected_entity(Some(self.entity));
            });
        }
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Room(self.room_id)
    }
}
