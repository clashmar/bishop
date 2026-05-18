pub mod game_map;
pub mod id_allocator;
pub mod startup_mode;

pub use game_map::*;
pub use id_allocator::*;
pub use startup_mode::*;

use crate::assets::{sprite_manager::SpriteManager, AssetRegistry};
use crate::ecs::ecs::Ecs;
#[cfg(feature = "editor")]
use crate::ecs::{get_root_entities_in_set, Entity};
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
        let current_id = self.current_world_id.unwrap_or_default();
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
        let current_id = self.current_world_id.unwrap_or_default();
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

    /// Mutable reference to the current world, or `None` if no worlds exist.
    pub fn current_world_mut(&mut self) -> Option<&mut World> {
        let current_id = self.current_world_id.unwrap_or_default();
        let idx = self.worlds.iter().position(|w| w.id == current_id);
        match idx {
            Some(i) => Some(&mut self.worlds[i]),
            None => self.worlds.iter_mut().next(),
        }
    }

    /// Immutable reference to the current world.
    pub fn current_world(&self) -> &World {
        let current_id = self.current_world_id.unwrap_or_default();
        self.worlds
            .iter()
            .find(|w| w.id == current_id)
            .unwrap_or_else(|| World::dummy())
    }

    /// Gets a mutable reference to a world from its id, or `None` if not found.
    pub fn get_world_mut(&mut self, world_id: WorldId) -> Option<&mut World> {
        self.worlds.iter_mut().find(|w| w.id == world_id)
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

        let entity_ids: HashSet<Entity> = room_ids
            .iter()
            .flat_map(|room_id| self.ecs.entities_in_room(*room_id).iter().copied())
            .collect();

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
        for world in &mut self.worlds {
            world.rebuild_room_grid();
        }
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
        for world in &mut self.worlds {
            world.rebuild_room_grid();
        }
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
mod tests;
