use crate::ecs::entity::Entity;
use crate::game::GameCtxMut;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Marks an entity as a named world entry point; arrivals use its `CurrentRoom` and `Transform`.
#[ecs_component(post_create = world_entry_post_create)]
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct WorldEntry {
    /// Entry point name, unique within its world.
    pub name: String,
}

impl WorldEntry {
    /// The reserved name for the default world entry point.
    pub const START: &'static str = "Start";
}

fn world_entry_post_create(entry: &mut WorldEntry, entity: &Entity, ctx: &mut GameCtxMut<'_>) {
    let Some(world) = ctx.world.as_deref() else {
        return;
    };
    let taken: HashSet<String> = ctx
        .ecs
        .get_store::<WorldEntry>()
        .data
        .iter()
        .filter(|(e, _)| *e != entity)
        .filter(|(e, _)| ctx.world_of_entity(**e) == Some(world.id))
        .map(|(_, e)| e.name.clone())
        .collect();
    if taken.contains(&entry.name) {
        entry.name = (2..)
            .map(|n| format!("{} {n}", entry.name))
            .find(|candidate| !taken.contains(candidate))
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetRegistry;
    use crate::assets::sprite_manager::SpriteManager;
    use crate::game::GameCtxMut;
    use crate::prefab::PrefabManager;
    use crate::scripting::script_manager::ScriptManager;
    use crate::worlds::world::{World, WorldId};
    use crate::worlds::{Room, RoomId};

    #[test]
    fn post_create_dedupes_world_entry_name_on_collision() {
        const ORIG: &str = "Cave";
        let mut ecs = crate::ecs::Ecs::default();
        let mut world = World::new(WorldId(1), String::new(), 16.0);
        world.add_room(Room { id: RoomId(1), ..Default::default() });

        ecs.create_entity()
            .with(WorldEntry { name: ORIG.to_string() })
            .with_current_room(RoomId(1))
            .finish();

        let mut sm = SpriteManager::default();
        let mut ar = AssetRegistry::default();
        let mut scm = ScriptManager::default();
        let pm = PrefabManager::default();
        let mut ctx = GameCtxMut {
            ecs: &mut ecs,
            world: Some(&mut world),
            world_directory: Vec::new(),
            room_world_map: [(RoomId(1), WorldId(1))].into_iter().collect(),
            asset_registry: &mut ar,
            sprite_manager: &mut sm,
            script_manager: &mut scm,
            prefab_manager: &pm,
        };

        let new_entity = ctx.ecs.create_entity().finish();
        let mut entry = WorldEntry { name: ORIG.to_string() };
        world_entry_post_create(&mut entry, &new_entity, &mut ctx);

        assert_eq!(entry.name, format!("{ORIG} 2"));
    }
}
