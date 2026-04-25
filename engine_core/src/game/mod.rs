pub mod game_map;
pub mod id_allocator;
pub mod startup_mode;

pub use game_map::*;
pub use id_allocator::*;
pub use startup_mode::*;

use crate::assets::{sprite_manager::SpriteManager, AssetRegistry};
use crate::ecs::ecs::Ecs;
#[cfg(feature = "editor")]
use crate::ecs::{get_root_entities_in_set, CurrentRoom, Entity};
use crate::engine_global::set_game_name;
use crate::onscreen_error;
use crate::prefab::{load_prefab_manager, PrefabManager};
use crate::scripting::script_manager::ScriptManager;
#[cfg(feature = "editor")]
use crate::worlds::room::RoomId;
use crate::worlds::world::*;
use crate::{storage::text_folder, text::TextManager};
use bishop::prelude::TextureLoader;
use mlua::Lua;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
#[cfg(feature = "editor")]
use std::collections::HashSet;
use uuid::Uuid;

#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Game {
    pub version: u32,
    /// Unique identifier of the game.
    pub id: Uuid,
    /// Human readable name of the game.
    pub name: String,
    /// Stores the game ECS.
    pub ecs: Ecs,
    /// All worlds belonging to this game instance.
    pub worlds: Vec<World>,
    /// Project-scoped authored asset registry.
    pub asset_registry: AssetRegistry,
    /// Asset manager for the game.
    pub sprite_manager: SpriteManager,
    /// Script manager for the game.
    pub script_manager: ScriptManager,
    /// Text manager for the game.
    #[serde(skip)]
    pub text_manager: TextManager,
    /// Prefab manager for the game.
    #[serde(skip)]
    pub prefab_manager: PrefabManager,
    /// Id of the currently active world. `None` means no world is active.
    /// `WorldId(0)` is used as a dummy when no worlds exist.
    pub current_world_id: Option<WorldId>,
    /// Top level map of the whole game.
    pub game_map: GameMap,
    #[serde(skip)]
    pub id_allocator: IdAllocator,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            version: 1,
            id: Uuid::new_v4(),
            name: String::new(),
            ecs: Ecs::default(),
            worlds: Vec::new(),
            asset_registry: AssetRegistry::default(),
            sprite_manager: SpriteManager::default(),
            script_manager: ScriptManager::default(),
            text_manager: TextManager::default(),
            prefab_manager: PrefabManager::default(),
            current_world_id: Some(WorldId(1)),
            game_map: GameMap::default(),
            id_allocator: IdAllocator::default(),
        }
    }
}

/// Bundles together common immutable game services.
pub struct GameCtx<'a> {
    pub ecs: &'a Ecs,
    pub world: &'a World,
    pub asset_registry: &'a AssetRegistry,
    pub sprite_manager: &'a SpriteManager,
    pub script_manager: &'a ScriptManager,
    pub prefab_manager: &'a PrefabManager,
}

/// Bundles together mutable game services used by editor and prefab workflows.
pub struct GameCtxMut<'a> {
    pub ecs: &'a mut Ecs,
    pub world: Option<&'a mut World>,
    pub asset_registry: &'a mut AssetRegistry,
    pub sprite_manager: &'a mut SpriteManager,
    pub script_manager: &'a mut ScriptManager,
    pub prefab_manager: &'a PrefabManager,
}

impl Game {
    /// Returns an immutable game context.
    pub fn ctx<'a>(&'a self) -> GameCtx<'a> {
        let current_id = self.current_world_id.unwrap_or(WorldId::default());
        let world = self
            .worlds
            .iter()
            .find(|w| w.id == current_id)
            .unwrap_or_else(|| World::dummy());

        GameCtx {
            ecs: &self.ecs,
            world,
            asset_registry: &self.asset_registry,
            sprite_manager: &self.sprite_manager,
            script_manager: &self.script_manager,
            prefab_manager: &self.prefab_manager,
        }
    }

    /// Returns a mutable game context.
    pub fn ctx_mut<'a>(&'a mut self) -> GameCtxMut<'a> {
        let current_id = self.current_world_id.unwrap_or(WorldId::default());
        let idx = self
            .worlds
            .iter()
            .position(|w| w.id == current_id)
            .unwrap_or(0);
        let world = self.worlds.get_mut(idx);

        GameCtxMut {
            ecs: &mut self.ecs,
            world,
            asset_registry: &mut self.asset_registry,
            sprite_manager: &mut self.sprite_manager,
            script_manager: &mut self.script_manager,
            prefab_manager: &self.prefab_manager,
        }
    }

    /// Mutable reference to the current world.
    pub fn current_world_mut(&mut self) -> &mut World {
        let current_id = self.current_world_id.unwrap_or(WorldId::default());
        let idx = self.worlds.iter().position(|w| w.id == current_id);
        match idx {
            Some(i) => &mut self.worlds[i],
            None => self.worlds.iter_mut().next().expect("No worlds in game."),
        }
    }

    /// Immutable reference to the current world.
    pub fn current_world(&self) -> &World {
        let current_id = self.current_world_id.unwrap_or(WorldId::default());
        self.worlds
            .iter()
            .find(|w| w.id == current_id)
            .unwrap_or_else(|| World::dummy())
    }

    /// Gets a mutable reference to a world from its id.
    pub fn get_world_mut(&mut self, world_id: WorldId) -> &mut World {
        self.worlds
            .iter_mut()
            .find(|w| w.id == world_id)
            .expect("World id not present in game.")
    }

    /// Add a new world and make it the active one.
    pub fn add_world(&mut self, world: World) {
        self.current_world_id = Some(world.id);
        self.worlds.push(world);
    }

    /// Switch the editor to a different world by its id.
    pub fn select_world(&mut self, id: WorldId) {
        if self.worlds.iter().any(|w| w.id == id) {
            self.current_world_id = Some(id);
        }
    }

    /// Deletes the world from the game.
    #[cfg(feature = "editor")]
    pub fn delete_world(&mut self, id: WorldId) {
        let room_ids: HashSet<RoomId> = self
            .worlds
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.rooms.iter().map(|r| r.id).collect())
            .unwrap_or_default();

        let entity_ids: HashSet<Entity> = {
            let store = self.ecs.get_store::<CurrentRoom>();
            store
                .data
                .iter()
                .filter(|(_, CurrentRoom(room))| room_ids.contains(room))
                .map(|(&entity, _)| entity)
                .collect()
        };

        let root_entities = get_root_entities_in_set(&self.ecs, &entity_ids);

        if let Some(pos) = self.worlds.iter().position(|w| w.id == id) {
            self.worlds.remove(pos);
        }

        if self.current_world_id == Some(id) {
            self.current_world_id = self
                .worlds
                .first()
                .map(|w| w.id)
                .or_else(|| Some(WorldId::default()));
        }

        let mut ctx = self.ctx_mut();
        for entity in root_entities {
            Ecs::remove_entity(&mut ctx, entity);
        }
    }

    /// Syncs all assets/scripts that belong to this game, sets the game name, and inits managers.
    pub fn initialize(&mut self, loader: &impl TextureLoader, lua: &Lua) {
        self.id_allocator = IdAllocator::from_game(self);
        set_game_name(self.name.clone());
        self.asset_registry.init_editor_metadata();
        SpriteManager::init_manager(loader, self);
        ScriptManager::init_manager(&self.asset_registry, &mut self.script_manager, lua);
        self.init_text_manager();
        self.reload_prefab_manager();
    }

    /// Initializes runtime state for the game without eagerly hydrating all textures.
    pub fn initialize_runtime(&mut self, lua: &Lua) {
        self.id_allocator = IdAllocator::from_game(self);
        set_game_name(self.name.clone());
        self.asset_registry.init_editor_metadata();
        SpriteManager::init_runtime_manager(self);
        ScriptManager::init_manager(&self.asset_registry, &mut self.script_manager, lua);
        self.init_text_manager();
        self.reload_prefab_manager();
    }

    /// Initializes the text manager with the correct path.
    pub fn init_text_manager(&mut self) {
        let text_root = text_folder();
        self.text_manager.set_text_root(text_root);
    }

    /// Reloads the prefab manager for the current game from disk.
    pub fn reload_prefab_manager(&mut self) {
        let game_name = self.name.clone();

        match load_prefab_manager(&game_name, &mut self.asset_registry) {
            Ok(prefab_manager) => {
                self.prefab_manager = prefab_manager;
            }
            Err(error) => {
                onscreen_error!("Failed to load prefabs: {error}");
            }
        }
    }
}

impl GameCtxMut<'_> {
    /// Mutable ECS access.
    pub fn ecs(&mut self) -> &mut Ecs {
        self.ecs
    }

    /// Mutable asset-manager access.
    pub fn sprite_manager(&mut self) -> &mut SpriteManager {
        self.sprite_manager
    }

    /// Mutable asset-registry access.
    pub fn asset_registry(&mut self) -> &mut AssetRegistry {
        self.asset_registry
    }

    /// Mutable script-manager access.
    pub fn script_manager(&mut self) -> &mut ScriptManager {
        self.script_manager
    }

    /// Mutable world access when this context is world-backed.
    pub fn current_world(&mut self) -> Option<&mut World> {
        self.world.as_deref_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{CurrentRoom, Ecs, Name};
    use crate::worlds::room::Room;

    #[test]
    fn game_ctx_mut_can_exist_without_a_current_world() {
        let mut ecs = Ecs::default();
        let mut asset_registry = AssetRegistry::default();
        let mut sprite_manager = SpriteManager::default();
        let mut script_manager = ScriptManager::default();
        let prefab_manager = PrefabManager::default();

        let ctx = GameCtxMut {
            ecs: &mut ecs,
            world: None,
            asset_registry: &mut asset_registry,
            sprite_manager: &mut sprite_manager,
            script_manager: &mut script_manager,
            prefab_manager: &prefab_manager,
        };

        assert!(ctx.world.is_none());
    }

    #[test]
    fn delete_world_sets_current_to_dummy_when_empty() {
        let mut game = Game::default();
        let world_id = game.id_allocator.allocate_world_id();
        game.add_world(World {
            id: world_id,
            ..Default::default()
        });
        game.delete_world(world_id);
        assert_eq!(game.current_world_id, Some(WorldId::default()));
    }

    #[test]
    fn delete_world_sets_current_to_remaining_world() {
        let mut game = Game::default();
        let w1 = game.id_allocator.allocate_world_id();
        let w2 = game.id_allocator.allocate_world_id();
        game.add_world(World {
            id: w1,
            name: "a".into(),
            ..Default::default()
        });
        game.add_world(World {
            id: w2,
            name: "b".into(),
            ..Default::default()
        });
        game.delete_world(w2);
        assert_eq!(game.current_world_id, Some(w1));
    }

    #[test]
    fn delete_world_removes_all_room_entities() {
        let mut game = Game::default();
        let world_id = game.id_allocator.allocate_world_id();
        let room_id = game.id_allocator.allocate_room_id();
        game.add_world(World {
            id: world_id,
            rooms: vec![Room {
                id: room_id,
                ..Default::default()
            }],
            ..Default::default()
        });

        let entity = game
            .ecs
            .create_entity()
            .with(CurrentRoom(room_id))
            .with(Name("test_entity".into()))
            .finish();

        game.delete_world(world_id);

        assert!(
            !game.ecs.has::<CurrentRoom>(entity),
            "CurrentRoom should be gone after world deletion"
        );
        assert!(
            !game.ecs.has::<Name>(entity),
            "Name should be gone after world deletion"
        );
    }

    #[test]
    fn delete_world_preserves_other_world_entities() {
        let mut game = Game::default();
        let world_a = game.id_allocator.allocate_world_id();
        let world_b = game.id_allocator.allocate_world_id();
        let room_a = game.id_allocator.allocate_room_id();
        let room_b = game.id_allocator.allocate_room_id();

        game.add_world(World {
            id: world_a,
            rooms: vec![Room {
                id: room_a,
                ..Default::default()
            }],
            ..Default::default()
        });
        game.add_world(World {
            id: world_b,
            rooms: vec![Room {
                id: room_b,
                ..Default::default()
            }],
            ..Default::default()
        });

        let entity_a = game.ecs.create_entity().with(CurrentRoom(room_a)).finish();
        let entity_b = game.ecs.create_entity().with(CurrentRoom(room_b)).finish();

        game.delete_world(world_a);

        assert!(
            !game.ecs.has::<CurrentRoom>(entity_a),
            "entity_a should be gone after its world is deleted"
        );
        assert!(
            game.ecs.has::<CurrentRoom>(entity_b),
            "entity_b should still exist after the other world is deleted"
        );
    }

    #[test]
    fn initialize_rebuilds_id_allocator() {
        let mut game = Game::default();
        let w1 = game.id_allocator.allocate_world_id();
        let r1 = game.id_allocator.allocate_room_id();
        game.add_world(World {
            id: w1,
            rooms: vec![Room {
                id: r1,
                ..Default::default()
            }],
            ..Default::default()
        });
        game.id_allocator = IdAllocator::default();
        game.id_allocator = IdAllocator::from_game(&game);
        assert!(game.id_allocator.allocate_world_id().0 > w1.0);
        let next_room = game.id_allocator.allocate_room_id();
        assert!(next_room.0 > r1.0);
    }

    #[cfg(feature = "editor")]
    #[test]
    fn reload_prefab_manager_keeps_existing_records_when_reload_fails() {
        use crate::constants::extensions;
        use crate::engine_global::set_game_name;
        use crate::prefab::{create_prefab, persist_prefab, PrefabId};
        use crate::storage::path_utils::prefabs_folder;
        use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};
        use std::path::PathBuf;

        let _lock = game_fs_test_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let test_folder = TestGameFolder::new("prefab_registry_no_partial_cleanup");
        set_game_name(test_folder.name());
        let first = create_prefab(PrefabId(3), "Bullet".to_string());
        let second = create_prefab(PrefabId(9), "Bullet".to_string());
        let stale_prefab_id = PrefabId(22);
        let stale_relative_path = PathBuf::from(format!("stale_prefab.{}", extensions::PREFAB));
        let mut game = Game {
            name: test_folder.name().to_string(),
            ..Default::default()
        };

        persist_prefab(test_folder.name(), &first, &AssetRegistry::default()).unwrap();
        game.reload_prefab_manager();
        std::fs::write(
            prefabs_folder().join(format!("bullet_copy.{}", extensions::PREFAB)),
            ron::to_string(&second).unwrap(),
        )
        .unwrap();
        game.asset_registry
            .register_asset_relative_path(stale_prefab_id, &stale_relative_path)
            .unwrap();

        let before = game.asset_registry.clone();
        let before_prefab_manager = game.prefab_manager.clone();

        game.reload_prefab_manager();

        assert_eq!(game.asset_registry, before);
        assert_eq!(game.prefab_manager, before_prefab_manager);
    }
}
