use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::scripting::event_tags::event_tag::EventTag;
use engine_core::worlds::*;

/// Undo-able command for editing the tags of a room.
#[derive(Debug)]
pub struct EditRoomTagsCmd {
    world_id: WorldId,
    room_id: RoomId,
    old_tags: Vec<EventTag>,
    new_tags: Vec<EventTag>,
}

impl EditRoomTagsCmd {
    pub fn new(world_id: WorldId, room_id: RoomId, old_tags: Vec<EventTag>, new_tags: Vec<EventTag>) -> Self {
        Self { world_id, room_id, old_tags, new_tags }
    }

    fn apply(world_id: WorldId, room_id: RoomId, tags: Vec<EventTag>) {
        with_editor(|editor| {
            if let Some(world) = editor.game.get_world_mut(world_id) {
                if let Some(room) = world.get_room_mut(room_id) {
                    room.tags = tags;
                }
            }
        });
    }
}

impl EditorCommand for EditRoomTagsCmd {
    fn execute(&mut self) {
        Self::apply(self.world_id, self.room_id, self.new_tags.clone());
    }

    fn undo(&mut self) {
        Self::apply(self.world_id, self.room_id, self.old_tags.clone());
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        matches!(current_mode, EditorMode::Room(_))
    }
}
