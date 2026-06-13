use super::super::tags_module::TagsPropertyModule;
use engine_core::worlds::room::Room;

/// Edits the tags of a room.
pub type RoomTagsModule = TagsPropertyModule<Room>;

impl RoomTagsModule {
    pub fn for_room() -> Self {
        Self::new(|room| &mut room.tags)
    }
}
