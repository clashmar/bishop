use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::scripting::event_tags::event_tag::EventTag;
use engine_core::worlds::*;

/// Undo-able command for editing the tags of a world.
#[derive(Debug)]
pub struct EditWorldTagsCmd {
    world_id: WorldId,
    old_tags: Vec<EventTag>,
    new_tags: Vec<EventTag>,
}

impl EditWorldTagsCmd {
    pub fn new(world_id: WorldId, old_tags: Vec<EventTag>, new_tags: Vec<EventTag>) -> Self {
        Self { world_id, old_tags, new_tags }
    }

    fn apply(world_id: WorldId, tags: Vec<EventTag>) {
        with_editor(|editor| {
            if let Some(world) = editor.game.get_world_mut(world_id) {
                world.tags = tags;
            }
        });
    }
}

impl EditorCommand for EditWorldTagsCmd {
    fn execute(&mut self) {
        Self::apply(self.world_id, self.new_tags.clone());
    }

    fn undo(&mut self) {
        Self::apply(self.world_id, self.old_tags.clone());
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        matches!(current_mode, EditorMode::Game | EditorMode::World(_))
    }
}
