// engine_core/src/script/script_manager.rs
use crate::assets::asset_manager::{AssetManager, IdPathAssetManager};
use crate::assets::asset_registry::AssetKey;
use crate::assets::AssetRegistry;
use crate::ecs::ScriptId;
use crate::ecs::entity::Entity;
use crate::scripting::event_bus::EventBus;
use crate::scripting::lua_constants::{lua_entity, lua_fields};
use crate::storage::path_utils::{scripts_folder, themes_folder};
use crate::*;
use mlua::Function;
use mlua::Lua;
use mlua::Table;
use mlua::Value;
use mlua::prelude::LuaResult;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

const SCRIPT_ASSET_KIND: &str = "Script";

/// Manages access to scripts and holds the Lua VM instance.
#[derive(Serialize, Deserialize, Default)]
pub struct ScriptManager {
    #[serde(skip)]
    /// Event bus used by the global script module.
    pub event_bus: EventBus,
    #[serde(skip)]
    /// Maps `ScriptId`'s to their `Table` definition.
    pub table_defs: HashMap<ScriptId, Table>,
    /// Script instances (per entity).
    #[serde(skip)]
    pub instances: HashMap<(Entity, ScriptId), Table>,
    #[serde(skip)]
    /// Maps ScriptId to optional update(dt) function.
    pub update_fns: HashMap<ScriptId, Function>,
    /// Init functions that need to be executed.
    #[serde(skip)]
    pub pending_inits: Vec<(Entity, ScriptId)>,
    /// Derived cache of all script ids to their paths.
    #[serde(skip)]
    pub script_id_to_path: HashMap<ScriptId, PathBuf>,
    #[serde(skip)]
    pub path_to_script_id: HashMap<PathBuf, ScriptId>,
    #[serde(skip)]
    /// Counter for script ids. Starts from 1.
    pub next_script_id: usize,
}

impl ScriptManager {
    /// Initializes a new script manager.
    pub async fn new() -> Self {
        Self {
            event_bus: EventBus::default(),
            table_defs: HashMap::new(),
            instances: HashMap::new(),
            update_fns: HashMap::new(),
            pending_inits: Vec::new(),
            script_id_to_path: HashMap::new(),
            path_to_script_id: HashMap::new(),
            next_script_id: 1,
        }
    }

    /// Returns the number of loaded script definitions.
    pub fn loaded_script_count(&self) -> usize {
        self.table_defs.len()
    }

    /// Returns the number of live script instances.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// Returns the number of registered script event listeners.
    pub fn event_listener_count(&self) -> usize {
        self.event_bus.listener_count()
    }

    /// Load the Lua table by id and return a reference to it.
    pub fn load_script_table(&mut self, lua: &Lua, id: ScriptId) -> LuaResult<&Table> {
        if self.table_defs.contains_key(&id) {
            return self
                .table_defs
                .get(&id)
                .ok_or_else(|| mlua::Error::RuntimeError("Table disappeared unexpectedly".into()));
        }

        let table = self.get_table_from_id(lua, id)?;

        if let Ok(update) = table.get::<_>(lua_entity::UPDATE) {
            self.update_fns.insert(id, update);
        }

        Ok(self.table_defs.entry(id).or_insert(table))
    }

    /// Returns the instance and whether the instance was freshly created.
    /// Runs `init` on the script if present.
    pub fn get_or_create_instance(
        &mut self,
        lua: &Lua,
        entity: Entity,
        script_id: ScriptId,
    ) -> LuaResult<(&Table, bool)> {
        let key = (entity, script_id);

        // Fast path: instance already exists (single lookup via entry API)
        if self.instances.contains_key(&key) {
            return Ok((
                self.instances.get(&key).ok_or_else(|| {
                    mlua::Error::RuntimeError("Instance disappeared unexpectedly".into())
                })?,
                false,
            ));
        }

        // Ensure table is loaded first
        self.load_script_table(lua, script_id)?;

        // Script definition
        let def = self
            .table_defs
            .get(&script_id)
            .ok_or_else(|| mlua::Error::RuntimeError("Script definition not loaded".into()))?
            .clone();

        // Create instance table
        let instance = lua.create_table()?;

        // Clone `public` values that can vary per instance
        if let Ok(public) = def.get::<Table>(lua_fields::PUBLIC) {
            let public_copy = lua.create_table()?;
            for pair in public.pairs::<Value, Value>() {
                let (k, v) = pair?;
                public_copy.set(k, v)?;
            }
            instance.set(lua_fields::PUBLIC, public_copy)?;
        }

        // Setup instance metatable, this makes sure that scripts
        // will check the script def for data not on the instance
        let mt = lua.create_table()?;
        mt.set("__index", def.clone())?;
        instance.set_metatable(Some(mt))?;

        // Insert and return reference using entry API
        Ok((self.instances.entry(key).or_insert(instance), true))
    }

    /// Returns a reference to the Lua table that represents the script instance.
    pub fn get_instance(&self, entity: Entity, script_id: ScriptId) -> LuaResult<&Table> {
        self.instances.get(&(entity, script_id)).ok_or_else(|| {
            mlua::Error::RuntimeError(format!(
                "Lua script instance not found for entity {:?}, script {:?}",
                entity, script_id
            ))
        })
    }

    /// Loads and returns a Lua table from disk by id.
    pub fn get_table_from_id(&mut self, lua: &Lua, id: ScriptId) -> LuaResult<Table> {
        let rel_path = self
            .script_id_to_path
            .get(&id)
            .ok_or_else(|| mlua::Error::RuntimeError(format!("Unknown script id: {:?}.", id)))?;

        let abs_path = scripts_folder().join(rel_path);

        let src =
            fs::read_to_string(abs_path).map_err(|e| mlua::Error::ExternalError(Arc::new(e)))?;

        let path_name = rel_path.display().to_string();

        let table: Table = lua.load(&src).set_name(path_name).eval()?;
        Ok(table)
    }

    /// Returns the id for `path`, loading it if necessary.
    pub fn get_or_load<P: AsRef<Path>>(
        &mut self,
        asset_registry: &mut AssetRegistry,
        path: P,
    ) -> Option<ScriptId> {
        let p = path.as_ref();
        if p.to_string_lossy().trim().is_empty() {
            return None;
        }

        if let Some(&id) = self.path_to_script_id.get(p) {
            return Some(id);
        }

        match self.init_script(asset_registry, p) {
            Ok(id) => Some(id),
            Err(err) => {
                onscreen_error!("{}", err);
                None
            }
        }
    }

    /// Load and initialize a script from the scripts folder.
    /// Returns the `ScriptId` for the script.
    pub fn init_script(
        &mut self,
        asset_registry: &mut AssetRegistry,
        rel_path: impl AsRef<Path>,
    ) -> Result<ScriptId, String> {
        let path = rel_path.as_ref().to_path_buf();

        if path.to_string_lossy().trim().is_empty() {
            return Err("Empty script path.".into());
        }

        // Already loaded, reuse the same id
        if let Some(&id) = self.path_to_script_id.get(&path) {
            return Ok(id);
        }

        if self.next_script_id == 0 {
            self.restore_next_script_id();
        }

        let id = match asset_registry.key_for_path(scripts_folder().join(&path)) {
            Some(AssetKey::Script(id)) => id,
            _ => ScriptId(self.next_script_id),
        };

        asset_registry
            .register_asset_relative_path(id, &path)
            .map_err(|error| error.to_string())?;

        self.path_to_script_id.insert(path.clone(), id);
        self.script_id_to_path.insert(id, path);

        self.restore_next_script_id();

        Ok(id)
    }

    /// Returns a path normalized relative to the game's scripts folder.
    pub fn normalize_path(&self, path: PathBuf) -> PathBuf {
        let scripts_dir = scripts_folder();
        path.strip_prefix(&scripts_dir)
            .unwrap_or_else(|_| &path)
            .to_path_buf()
    }

    /// Initialize script manager state from the asset registry and Lua runtime.
    pub fn init_manager(
        asset_registry: &AssetRegistry,
        script_manager: &mut ScriptManager,
        lua: &Lua,
    ) {
        Self::init_editor_metadata(asset_registry, script_manager);
        Self::load_to_package(lua);
    }

    /// Initializes editor script metadata without requiring a Lua context.
    pub fn init_editor_metadata(asset_registry: &AssetRegistry, script_manager: &mut ScriptManager) {
        script_manager.rebuild_path_cache_from_registry(asset_registry);
        script_manager.restore_next_script_id();
    }

    /// Load all .lua files to the package.path.
    pub fn load_to_package(lua: &Lua) {
        let scripts_dir = scripts_folder().to_string_lossy().replace('\\', "/");
        let themes_dir = themes_folder().to_string_lossy().replace('\\', "/");

        onscreen_debug!("package.path loaded from: {} {}", scripts_dir, themes_dir);

        let add_path = format!(
            r#"
            local p = package.path
            package.path = p .. ';{scripts_dir}/?.lua;{scripts_dir}/?/init.lua;{themes_dir}/?.lua'
            "#,
        );

        lua.load(&add_path).exec().expect("Cannot set package.path");
    }

    /// Calculates the next script id.
    pub fn restore_next_script_id(&mut self) {
        let used: HashSet<_> = self
            .script_id_to_path
            .keys()
            .map(|sid| sid.0)
            .filter(|&id| id != 0)
            .collect();

        let mut candidate = 1usize;

        // Scan through until an unused id is found
        while used.contains(&candidate) {
            candidate += 1;
        }
        self.next_script_id = candidate;
    }

    /// Returns the number of registered script ids.
    pub fn registered_id_count(&self) -> usize {
        self.script_id_to_path.len()
    }

    /// Returns the registered relative path for a script id.
    pub fn path_for_id(&self, script_id: ScriptId) -> Option<&Path> {
        self.script_id_to_path.get(&script_id).map(PathBuf::as_path)
    }

    pub fn reload(&mut self, lua: &Lua, entity: Entity, id: ScriptId) -> LuaResult<&Table> {
        self.table_defs.remove(&id);
        self.instances.remove(&(entity, id));
        self.update_fns.remove(&id);
        self.load_script_table(lua, id)
    }

    pub fn unload(&mut self, entity: Entity, script_id: ScriptId) {
        // Remove any event listeners registered by this entity's script
        self.event_bus.remove_entity_listeners(entity);

        self.instances
            .retain(|(ent, _script_id), _table| *ent != entity);
        
        if script_id.0 != 0 && !self.instances.keys().any(|(_, id)| *id == script_id) {
            self.table_defs.remove(&script_id);
            self.update_fns.remove(&script_id);
        }
    }

    /// Change the script for an entity.
    pub fn change_script(&mut self, entity: Entity, old_id: &mut ScriptId, new_id: ScriptId) {
        if *old_id == new_id {
            return;
        }

        // Update old script counter
        if old_id.0 != 0 {
            self.instances.remove(&(entity, *old_id));
            if !self.instances.keys().any(|(_, id)| *id == *old_id) {
                self.table_defs.remove(old_id);
                self.update_fns.remove(old_id);
            }
        }

        *old_id = new_id;
    }

    fn rebuild_path_cache_from_registry(&mut self, asset_registry: &AssetRegistry) {
        self.script_id_to_path.clear();
        self.path_to_script_id.clear();

        for record_key in asset_registry.records().keys().copied() {
            let crate::assets::AssetKey::Script(script_id) = record_key else {
                continue;
            };
            let Some(relative_path) = asset_registry.relative_path(script_id) else {
                continue;
            };

            self.path_to_script_id.insert(relative_path.clone(), script_id);
            self.script_id_to_path.insert(script_id, relative_path);
        }
    }

}

impl AssetManager for ScriptManager {
    fn editor_metadata_snapshot(&self) -> Self {
        Self::default()
    }

    fn merge_editor_metadata_from(&mut self, source: &Self) -> std::io::Result<()> {
        let _ = source;
        Ok(())
    }
}

impl IdPathAssetManager for ScriptManager {
    type AssetId = ScriptId;

    fn asset_kind() -> &'static str {
        SCRIPT_ASSET_KIND
    }

    fn id_to_path(&self) -> &HashMap<Self::AssetId, PathBuf> {
        &self.script_id_to_path
    }

    fn id_to_path_mut(&mut self) -> &mut HashMap<Self::AssetId, PathBuf> {
        &mut self.script_id_to_path
    }

    fn path_to_id(&self) -> &HashMap<PathBuf, Self::AssetId> {
        &self.path_to_script_id
    }

    fn path_to_id_mut(&mut self) -> &mut HashMap<PathBuf, Self::AssetId> {
        &mut self.path_to_script_id
    }

    fn rebuild_editor_metadata(&mut self) {
        self.restore_next_script_id();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::asset_registry::AssetKey;
    use crate::constants::paths;

    #[test]
    fn get_or_load_registers_new_script_path_in_asset_registry() {
        let mut registry = AssetRegistry::default();
        let mut script_manager = ScriptManager::default();
        let path = PathBuf::from("player.lua");

        let result = script_manager.get_or_load(&mut registry, &path);

        assert_eq!(result, Some(ScriptId(1)));
        assert_eq!(
            registry.key_for_path(PathBuf::from(paths::SCRIPTS_FOLDER).join(&path)),
            Some(AssetKey::Script(ScriptId(1)))
        );
        assert_eq!(script_manager.path_to_script_id.get(&path), Some(&ScriptId(1)));
    }
}
