mod world_exit;

use crate::engine::game_instance::GameInstance;
use engine_core::ecs::component_registry::COMPONENTS;
use engine_core::ecs::entity::Entity;

/// A component's response to an interact event.
pub struct InteractEntry {
    /// Must match the component's `TYPE_NAME`.
    pub type_name: &'static str,
    /// Called when the entity receives an interact event.
    pub on_interact: fn(entity: Entity, game_instance: &GameInstance),
}

inventory::collect!(InteractEntry);

/// Dispatches `on_interact` for every component on `entity` that has registered a handler.
pub fn handle_interactions(entity: Entity, game_instance: &GameInstance) {
    let ecs = &game_instance.game.ecs;
    for entry in inventory::iter::<InteractEntry> {
        let has = COMPONENTS
            .iter()
            .find(|r| r.type_name == entry.type_name)
            .is_some_and(|r| (r.has)(ecs, entity));
        if has {
            (entry.on_interact)(entity, game_instance);
        }
    }
}
