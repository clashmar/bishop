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
    /// Whether this entry is the world's start point.
    /// At most one entry per world may have this set.
    #[serde(default)]
    pub is_start: bool,
}

impl WorldEntry {
    /// Reserved entry name; blocked from manual use and displayed for unnamed start entries.
    pub const START_NAME: &str = "Start";

    /// Display label given raw entry fields.
    pub fn display_name(name: &str, is_start: bool) -> &str {
        if name.is_empty() && is_start {
            Self::START_NAME
        } else if name.is_empty() {
            "(unnamed)"
        } else {
            name
        }
    }
}

fn world_entry_post_create(entry: &mut WorldEntry, entity: &Entity, ctx: &mut GameCtxMut<'_>) {
    let Some(world) = ctx.world.as_deref() else {
        return;
    };

    if !entry.is_start {
        let has_start = ctx
            .ecs
            .get_store::<WorldEntry>()
            .data
            .iter()
            .filter(|(e, _)| *e != entity)
            .filter(|(e, _)| ctx.world_of_entity(**e) == Some(world.id))
            .any(|(_, e)| e.is_start);
        if !has_start {
            entry.is_start = true;
        }
    }

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
            .with(WorldEntry { name: ORIG.to_string(), ..Default::default() })
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
        let mut entry = WorldEntry { name: ORIG.to_string(), ..Default::default() };
        world_entry_post_create(&mut entry, &new_entity, &mut ctx);

        assert_eq!(entry.name, format!("{ORIG} 2"));
    }

    #[test]
    fn world_entry_is_start_defaults_to_false() {
        let entry = WorldEntry::default();
        assert!(!entry.is_start);
    }

    #[test]
    fn world_entry_with_is_start_serializes_round_trip() {
        let entry = WorldEntry {
            name: "Cave".to_string(),
            is_start: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let roundtripped: WorldEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.name, "Cave");
        assert!(roundtripped.is_start);
    }

    #[test]
    fn post_create_defaults_to_start_when_no_other_start_in_world() {
        let mut ecs = crate::ecs::Ecs::default();
        let mut world = World::new(WorldId(1), String::new(), 16.0);
        world.add_room(Room { id: RoomId(1), ..Default::default() });

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
        let mut entry = WorldEntry { name: "Cave".to_string(), ..Default::default() };
        world_entry_post_create(&mut entry, &new_entity, &mut ctx);

        assert!(entry.is_start);
    }

    #[test]
    fn display_name_for_covers_all_cases() {
        assert_eq!(WorldEntry::display_name("", true), WorldEntry::START_NAME);
        assert_eq!(WorldEntry::display_name("", false), "(unnamed)");
        assert_eq!(WorldEntry::display_name("Cave", false), "Cave");
        assert_eq!(WorldEntry::display_name("Cave", true), "Cave");
    }

    #[test]
    fn post_create_does_not_default_to_start_when_other_start_exists() {
        let mut ecs = crate::ecs::Ecs::default();
        let mut world = World::new(WorldId(1), String::new(), 16.0);
        world.add_room(Room { id: RoomId(1), ..Default::default() });

        ecs.create_entity()
            .with(WorldEntry { name: WorldEntry::START_NAME.to_string(), is_start: true })
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
        let mut entry = WorldEntry { name: "Cave".to_string(), ..Default::default() };
        world_entry_post_create(&mut entry, &new_entity, &mut ctx);

        assert!(!entry.is_start);
    }
}