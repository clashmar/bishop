use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::commands::scene::context::{uses_prefab_context, with_scene_ctx};
use crate::with_editor;
use engine_core::prelude::*;
/// Undo-able command for deleting an entity and its children.
#[derive(Debug)]
pub struct DeleteEntityCmd {
    pub entity: Entity,
    pub mode: EditorMode,
    pub saved: Option<GroupSnapshot>,
    pub cleared_prefab_root: bool,
}

impl DeleteEntityCmd {
    pub fn new(entity: Entity, mode: EditorMode) -> Self {
        Self {
            entity,
            mode,
            saved: None,
            cleared_prefab_root: false,
        }
    }
}

impl EditorCommand for DeleteEntityCmd {
    fn execute(&mut self) {
        let prefab_mode = uses_prefab_context(self.mode);
        with_editor(|editor| {
            let mut deleted_entities = Vec::new();
            with_scene_ctx(editor, self.mode, |ctx| {
                self.saved = Some(capture_subtree(ctx.ecs(), self.entity));
                deleted_entities = self
                    .saved
                    .as_ref()
                    .map(|saved| saved.iter().map(|snapshot| snapshot.entity).collect())
                    .unwrap_or_default();
                Ecs::remove_entity(ctx, self.entity);
            });
            if prefab_mode {
                let prefab_editor = editor
                    .prefab_editor
                    .as_mut()
                    .expect("Prefab editor missing");
                self.cleared_prefab_root = prefab_editor.root_entity == Some(self.entity);
                prefab_editor.clear_deleted_entities(&deleted_entities);
            } else {
                editor.room_editor.set_selected_entity(None);
            }
        });
    }

    fn undo(&mut self) {
        if let Some(saved) = self.saved.take() {
            let prefab_mode = uses_prefab_context(self.mode);
            with_editor(|editor| {
                with_scene_ctx(editor, self.mode, |ctx| restore_subtree(ctx, &saved));
                if prefab_mode {
                    let prefab_editor = editor
                        .prefab_editor
                        .as_mut()
                        .expect("Prefab editor missing");
                    if self.cleared_prefab_root {
                        prefab_editor.restore_deleted_root(self.entity);
                    } else {
                        prefab_editor.set_selected_entity(Some(self.entity));
                    }
                } else {
                    editor.room_editor.set_selected_entity(Some(self.entity));
                }
            });
        }
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        self.mode == current_mode
    }
}
