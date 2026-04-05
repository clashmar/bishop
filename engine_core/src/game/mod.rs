// engine_core/src/game/mod.rs

pub mod game_map;
pub mod startup_mode;

pub use game_map::*;
pub use startup_mode::*;

use crate::assets::asset_manager::AssetManager;
use crate::ecs::ecs::Ecs;
use crate::engine_global::set_game_name;
use crate::onscreen_error;
use crate::prefab::{PrefabLibrary, load_prefab_library};
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
    /// Asset manager for the game.
    pub asset_manager: AssetManager,
    /// Script manager for the game.
    pub script_manager: ScriptManager,
    /// Text manager for the game.
    #[serde(skip)]
    pub text_manager: TextManager,
    /// In-memory prefab library loaded from disk for the current game.
    #[serde(skip)]
    pub prefab_library: PrefabLibrary,
    /// Id of the currently active world.
    pub current_world_id: WorldId, // TODO: Change this to an option
    /// Top level map of the whole game.
    pub game_map: GameMap,
    /// Counter for allocating globally unique room Ids.
    pub next_room_id: usize,
}

/// Bundles together common immutable systems.
pub struct GameCtx<'a> {
    pub ecs: &'a Ecs,
    pub cur_world: &'a World,
    pub asset_manager: &'a AssetManager,
    pub script_manager: &'a ScriptManager,
}

/// Bundles together common mutable systems.
pub struct GameCtxMut<'a> {
    pub ecs: &'a mut Ecs,
    pub cur_world: Option<&'a mut World>,
    pub asset_manager: &'a mut AssetManager,
    pub script_manager: &'a mut ScriptManager,
    /// Read-only prefab library for UI and editor lookups.
    pub prefab_library: &'a PrefabLibrary,
}

/// Bundles together mutable services used by editor and prefab entity workflows.
pub struct ServicesCtxMut<'a> {
    pub ecs: &'a mut Ecs,
    pub world: Option<&'a mut World>,
    pub asset_manager: &'a mut AssetManager,
    pub script_manager: &'a mut ScriptManager,
    /// Read-only prefab library for UI and editor lookups.
    pub prefab_library: &'a PrefabLibrary,
}

/// Mutable engine services used by hooks, prefab helpers, and editor entity workflows.
pub trait EngineCtxMut {
    /// Mutable ECS access.
    fn ecs(&mut self) -> &mut Ecs;

    /// Mutable asset-manager access.
    fn asset_manager(&mut self) -> &mut AssetManager;

    /// Mutable script-manager access.
    fn script_manager(&mut self) -> &mut ScriptManager;

    /// Mutable world access when this context is world-backed.
    fn current_world(&mut self) -> Option<&mut World>;
}

impl Game {
    /// Returns an immutable game context.
    pub fn ctx<'a>(&'a self) -> GameCtx<'a> {
        let cur_world = self
            .worlds
            .iter()
            .find(|w| w.id == self.current_world_id)
            .expect("There must be a current world.");

        GameCtx {
            ecs: &self.ecs,
            cur_world,
            asset_manager: &self.asset_manager,
            script_manager: &self.script_manager,
        }
    }

    /// Returns a mutable game context.
    pub fn ctx_mut<'a>(&'a mut self) -> GameCtxMut<'a> {
        let cur_world = self
            .worlds
            .iter_mut()
            .find(|w| w.id == self.current_world_id)
            .expect("There must be a current world.");

        GameCtxMut {
            ecs: &mut self.ecs,
            cur_world: Some(cur_world),
            asset_manager: &mut self.asset_manager,
            script_manager: &mut self.script_manager,
            prefab_library: &self.prefab_library,
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
        AssetManager::init_manager(loader, self);
        ScriptManager::init_manager(self, lua);
        self.init_text_manager();
        self.reload_prefab_library();
    }

    /// Initializes runtime state for the game without eagerly hydrating all textures.
    pub fn initialize_runtime(&mut self, lua: &Lua) {
        set_game_name(self.name.clone());
        AssetManager::init_runtime_manager(self);
        ScriptManager::init_manager(self, lua);
        self.init_text_manager();
        self.reload_prefab_library();
    }

    /// Initializes the text manager with the correct path.
    pub fn init_text_manager(&mut self) {
        let text_root = text_folder();
        self.text_manager.set_text_root(text_root);
    }

    /// Reloads the prefab library for the current game from disk.
    pub fn reload_prefab_library(&mut self) {
        match load_prefab_library(&self.name) {
            Ok(prefab_library) => {
                self.prefab_library = prefab_library;
            }
            Err(error) => {
                onscreen_error!("Failed to load prefabs: {error}");
                self.prefab_library = PrefabLibrary::default();
            }
        }
    }

    /// Allocates a globally unique room ID.
    pub fn allocate_room_id(&mut self) -> RoomId {
        self.next_room_id += 1;
        RoomId(self.next_room_id)
    }
}

impl<'a> GameCtxMut<'a> {
    /// Returns a mutable services context without requiring room-specific access.
    pub fn services_ctx_mut(&mut self) -> ServicesCtxMut<'_> {
        ServicesCtxMut {
            ecs: self.ecs,
            world: self.cur_world.as_deref_mut(),
            asset_manager: self.asset_manager,
            script_manager: self.script_manager,
            prefab_library: self.prefab_library,
        }
    }
}

impl EngineCtxMut for GameCtxMut<'_> {
    fn ecs(&mut self) -> &mut Ecs {
        self.ecs
    }

    fn asset_manager(&mut self) -> &mut AssetManager {
        self.asset_manager
    }

    fn script_manager(&mut self) -> &mut ScriptManager {
        self.script_manager
    }

    fn current_world(&mut self) -> Option<&mut World> {
        self.cur_world.as_deref_mut()
    }
}

impl EngineCtxMut for ServicesCtxMut<'_> {
    fn ecs(&mut self) -> &mut Ecs {
        self.ecs
    }

    fn asset_manager(&mut self) -> &mut AssetManager {
        self.asset_manager
    }

    fn script_manager(&mut self) -> &mut ScriptManager {
        self.script_manager
    }

    fn current_world(&mut self) -> Option<&mut World> {
        self.world.as_deref_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_ctx_mut_can_exist_without_a_current_world() {
        let mut ecs = Ecs::default();
        let mut asset_manager = AssetManager::default();
        let mut script_manager = ScriptManager::default();

        let ctx = GameCtxMut {
            ecs: &mut ecs,
            cur_world: None,
            asset_manager: &mut asset_manager,
            script_manager: &mut script_manager,
            prefab_library: &PrefabLibrary::default(),
        };

        assert!(ctx.cur_world.is_none());
    }
}
