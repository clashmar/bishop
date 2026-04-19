// engine_core/src/game/mod.rs

pub mod game_map;
pub mod startup_mode;

pub use game_map::*;
pub use startup_mode::*;

use crate::assets::{sprite_manager::SpriteManager, AssetRegistry};
use crate::ecs::ecs::Ecs;
use crate::engine_global::set_game_name;
use crate::onscreen_error;
use crate::prefab::{load_prefab_manager, PrefabManager};
use crate::scripting::script_manager::ScriptManager;
use crate::worlds::room::RoomId;
use crate::worlds::world::*;
use crate::{storage::text_folder, text::TextManager};
use bishop::prelude::TextureLoader;
use mlua::Lua;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use uuid::Uuid;

#[serde_as]
#[derive(Serialize, Deserialize, Default)]
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
    /// Id of the currently active world.
    pub current_world_id: WorldId, // TODO: Change this to an option
    /// Top level map of the whole game.
    pub game_map: GameMap,
    /// Counter for allocating globally unique room Ids.
    pub next_room_id: usize,
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
        let world = self
            .worlds
            .iter()
            .find(|w| w.id == self.current_world_id)
            .expect("There must be a current world.");

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
        let world = self
            .worlds
            .iter_mut()
            .find(|w| w.id == self.current_world_id)
            .expect("There must be a current world.");

        GameCtxMut {
            ecs: &mut self.ecs,
            world: Some(world),
            asset_registry: &mut self.asset_registry,
            sprite_manager: &mut self.sprite_manager,
            script_manager: &mut self.script_manager,
            prefab_manager: &self.prefab_manager,
        }
    }

    /// Mutable reference to the current world.
    pub fn current_world_mut(&mut self) -> &mut World {
        self.worlds
            .iter_mut()
            .find(|w| w.id == self.current_world_id)
            .expect("Current world id not present in game.")
    }

    /// Immutable reference to the current world.
    pub fn current_world(&self) -> &World {
        self.worlds
            .iter()
            .find(|w| w.id == self.current_world_id)
            .expect("Current world id not present in game.")
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
        self.current_world_id = world.id;
        self.worlds.push(world);
    }

    /// Switch the editor to a different world by its id.
    pub fn select_world(&mut self, id: WorldId) {
        if self.worlds.iter().any(|w| w.id == id) {
            self.current_world_id = id;
        }
    }

    /// Deletes the world from the game.
    pub fn delete_world(&mut self, id: WorldId) {
        if let Some(pos) = self.worlds.iter().position(|w| w.id == id) {
            self.worlds.swap_remove(pos);
        }

        if self.current_world_id == id {
            self.current_world_id = self
                .worlds
                .first()
                .map(|w| w.id)
                .unwrap_or(WorldId(Uuid::nil()));
        }
    }

    /// Syncs all assets/scripts that belong to this game, sets the game name, and inits managers.
    pub fn initialize(&mut self, loader: &impl TextureLoader, lua: &Lua) {
        set_game_name(self.name.clone());
        self.asset_registry.init_editor_metadata();
        SpriteManager::init_manager(loader, self);
        ScriptManager::init_manager(&self.asset_registry, &mut self.script_manager, lua);
        self.init_text_manager();
        self.reload_prefab_manager();
    }

    /// Initializes runtime state for the game without eagerly hydrating all textures.
    pub fn initialize_runtime(&mut self, lua: &Lua) {
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

    /// Allocates a globally unique room ID.
    pub fn allocate_room_id(&mut self) -> RoomId {
        self.next_room_id += 1;
        RoomId(self.next_room_id)
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
}
