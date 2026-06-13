use super::super::tags_module::TagsPropertyModule;
use engine_core::worlds::world::World;

/// Edits the tags of a world.
pub type WorldTagsModule = TagsPropertyModule<World>;

impl WorldTagsModule {
    pub fn for_world() -> Self {
        Self::new(|world| &mut world.tags)
    }
}
