// engine_core/src/script/script_manager.rs
use crate::ecs::entity::Entity;
use crate::game::Game;
use crate::scripting::event_bus::EventBus;
use crate::scripting::lua_constants::*;
use crate::scripting::script::*;
use crate::storage::path_utils::scripts_folder;
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
    /// Persistent map of all script ids to their paths.
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    pub script_id_to_path: HashMap<ScriptId, PathBuf>,
    #[serde(skip)]
    pub path_to_script_id: HashMap<PathBuf, ScriptId>,
    #[serde(skip)]
    /// Counter for script ids. Starts from 1.
    pub next_script_id: usize,
    /// How many entities are using a script.
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    ref_counts: HashMap<ScriptId, usize>,
    /// Script ids whose path mappings should be removed on exit.
    #[cfg(feature = "editor")]
    #[serde(skip)]
    pending_path_removal: HashSet<ScriptId>,
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
            ref_counts: HashMap::new(),
            #[cfg(feature = "editor")]
            pending_path_removal: HashSet::new(),
        }
    }

    /// Increment reference count for a script.
    pub fn increment_ref(&mut self, script_id: ScriptId) {
        if script_id.0 == 0 {
            return;
        }

        *self.ref_counts.entry(script_id).or_insert(0) += 1;

        #[cfg(feature = "editor")]
        {
            self.pending_path_removal.remove(&script_id);
        }
    }

    /// Decrement reference count for a script, and clean up if it reaches zero.
    fn decrement_ref(&mut self, script_id: ScriptId) {
        if script_id.0 == 0 {
            return;
        }

        if let Some(count) = self.ref_counts.get_mut(&script_id) {
            *count = count.saturating_sub(1);

            if *count == 0 {
                self.ref_counts.remove(&script_id);
                self.table_defs.remove(&script_id);
                self.update_fns.remove(&script_id);

                #[cfg(feature = "editor")]
                {
                    self.pending_path_removal.insert(script_id);
                }
            }
        }
    }

    /// Remove path mappings for all scripts with a zero ref count.
    /// Call this before serializing game data on exit.
    #[cfg(feature = "editor")]
    pub fn flush_pending_removals(&mut self) {
        for id in self.pending_path_removal.drain() {
            if let Some(path) = self.script_id_to_path.remove(&id) {
                self.path_to_script_id.remove(&path);
            }
        }
    }

    /// Get the reference count for a script.
    pub fn get_ref_count(&self, script_id: ScriptId) -> usize {
        self.ref_counts.get(&script_id).copied().unwrap_or(0)
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

        if let Ok(update) = table.get::<_>(UPDATE) {
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
        if let Ok(public) = def.get::<Table>(PUBLIC) {
            let public_copy = lua.create_table()?;
            for pair in public.pairs::<Value, Value>() {
                let (k, v) = pair?;
                public_copy.set(k, v)?;
            }
            instance.set(PUBLIC, public_copy)?;
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
    pub fn get_or_load<P: AsRef<Path>>(&mut self, path: P) -> Option<ScriptId> {
        let p = path.as_ref();
        if p.to_string_lossy().trim().is_empty() {
            return None;
        }

        if let Some(&id) = self.path_to_script_id.get(p) {
            return Some(id);
        }

        match self.init_script(p) {
            Ok(id) => Some(id),
            Err(err) => {
                onscreen_error!("{}", err);
                None
            }
        }
    }

    /// Load and initialize a script from the scripts folder.
    /// Returns the `ScriptId` for the script.
    pub fn init_script(&mut self, rel_path: impl AsRef<Path>) -> Result<ScriptId, String> {
        let path = rel_path.as_ref().to_path_buf();

        if path.to_string_lossy().trim().is_empty() {
            return Err("Empty script path.".into());
        }

        // Already loaded, reuse the same id
        if let Some(&id) = self.path_to_script_id.get(&path) {
            return Ok(id);
        }

        // Set and calculate the next script id
        let id = ScriptId(self.next_script_id);

        // Store everything
        self.path_to_script_id.insert(path.clone(), id);
        self.script_id_to_path.insert(id, path);

        // Restore after inserting
        self.restore_next_id();

        Ok(id)
    }

    /// Returns a path normalized relative to the game's scripts folder.
    pub fn normalize_path(&self, path: PathBuf) -> PathBuf {
        let scripts_dir = scripts_folder();
        path.strip_prefix(&scripts_dir)
            .unwrap_or_else(|_| &path)
            .to_path_buf()
    }

    /// Initialize all scripts for the game.
    pub fn init_manager(game: &mut Game, lua: &Lua) {
        Self::load_to_package(lua);

        // Calculate the next id from the existing map
        game.script_manager.restore_next_id();

        // Repopulate reverse map
        let scripts: Vec<(ScriptId, PathBuf)> = game
            .script_manager
            .script_id_to_path
            .iter()
            .map(|(id, path)| (*id, path.clone()))
            .collect();

        for (id, path) in scripts {
            game.script_manager
                .path_to_script_id
                .insert(path.clone(), id);
        }
    }

    /// Load all .lua files to the package.path
    fn load_to_package(lua: &Lua) {
        let dir = scripts_folder().to_string_lossy().replace('\\', "/");

        onscreen_debug!("package.path loaded from: {}", dir);

        let add_path = format!(
            r#"
            local p = package.path
            package.path = p .. ';{dir}/?.lua;{dir}/?/init.lua'
            "#,
        );

        lua.load(&add_path).exec().expect("Cannot set package.path");
    }

    /// Calculates the next script id.
    fn restore_next_id(&mut self) {
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
        self.decrement_ref(script_id)
    }

    /// Change the script for an entity.
    pub fn change_script(&mut self, entity: Entity, old_id: &mut ScriptId, new_id: ScriptId) {
        if *old_id == new_id {
            return;
        }

        // Update old script counter
        if old_id.0 != 0 {
            self.instances.remove(&(entity, *old_id));
            self.decrement_ref(*old_id);
        }

        *old_id = new_id;
        self.increment_ref(new_id)
    }
}
