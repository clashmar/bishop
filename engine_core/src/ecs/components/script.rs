use crate::ecs::entity::Entity;
use crate::game::EngineCtxMut;
use crate::scripting::helpers::{read_script_field, write_script_field};
use crate::scripting::lua_constants::PUBLIC;
use crate::scripting::script_manager::ScriptManager;
use ecs_component::ecs_component;
use mlua::prelude::LuaResult;
use mlua::Lua;
use mlua::Table;
use mlua::Value;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

/// Opaque handle that the script manager gives out. Default/Unset is 0.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize, Default,
)]
pub struct ScriptId(pub usize);

/// One field that can be edited in the inspector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptField {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
}

/// The script data that the editor shows.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptData {
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    pub fields: HashMap<String, ScriptField>,
}

/// The script component that lives on an entity.
#[ecs_component(post_create = post_create, post_remove = post_remove)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Script {
    /// Id stored by the script manager.
    pub script_id: ScriptId,
    /// The public fields that the inspector can edit.
    pub data: ScriptData,
}

impl Script {
    /// Loads the table from ScriptManager and updates ScriptData.
    pub fn load(
        &mut self,
        lua: &Lua,
        script_manager: &mut ScriptManager,
        entity: Entity,
    ) -> LuaResult<()> {
        if self.script_id.0 == 0 {
            // Script hasn't been set yet
            self.data.fields.clear();
            return Ok(());
        }

        // Get or create the per-entity instance
        let (instance, _created) =
            script_manager.get_or_create_instance(lua, entity, self.script_id)?;

        // Determine the public fields table
        let public: Table = match instance.get::<Option<Table>>(PUBLIC)? {
            Some(t) => t,
            None => instance.clone(),
        };

        let mut fields = HashMap::new();

        for pair in public.pairs::<String, Value>() {
            let (name, value) = pair?;
            if let Some(field) = read_script_field(&name, value)? {
                fields.insert(name, field);
            }
        }

        // Remove any stale fields
        self.data.fields.retain(|name, _| fields.contains_key(name));
        // Add or update fields
        for (name, field) in fields {
            self.data.fields.entry(name).or_insert(field);
        }

        // Sync current values back to Lua
        self.sync_to_lua(lua, script_manager, entity)?;

        Ok(())
    }

    /// Sync the current ScriptData back to Lua table.
    pub fn sync_to_lua(
        &self,
        lua: &Lua,
        script_manager: &mut ScriptManager,
        entity: Entity,
    ) -> LuaResult<()> {
        if self.script_id.0 == 0 {
            return Ok(());
        }

        let (instance, _created) =
            script_manager.get_or_create_instance(lua, entity, self.script_id)?;

        self.sync_to_lua_with_instance(lua, instance)
    }

    /// Sync the current ScriptData to an already-retrieved Lua instance.
    /// Use this when you already have the instance to avoid redundant lookups.
    pub fn sync_to_lua_with_instance(&self, lua: &Lua, instance: &Table) -> LuaResult<()> {
        let public = instance
            .get::<Option<Table>>(PUBLIC)?
            .unwrap_or_else(|| instance.clone());

        for (name, field) in &self.data.fields {
            write_script_field(lua, &public, name, field)?;
        }
        Ok(())
    }
}

fn post_create(script: &mut Script, _entity: &Entity, ctx: &mut dyn EngineCtxMut) {
    ctx.script_manager().increment_ref(script.script_id)
}

fn post_remove(script: &mut Script, entity: &Entity, ctx: &mut dyn EngineCtxMut) {
    ctx.script_manager().unload(*entity, script.script_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::lua_constants::{POSITION, PUBLIC};

    #[test]
    fn load_reads_named_vec_tables_and_syncs_named_vec_tables() {
        let lua = Lua::new();
        let mut script_manager = ScriptManager::default();
        let def = lua.create_table().unwrap();
        let public = lua.create_table().unwrap();
        let position = lua.create_table().unwrap();
        let color = lua.create_table().unwrap();

        position.set("x", 12.5).unwrap();
        position.set("y", -3.0).unwrap();
        color.set("x", 0.25).unwrap();
        color.set("y", 0.5).unwrap();
        color.set("z", 0.75).unwrap();
        public.set(POSITION, position).unwrap();
        public.set("color", color).unwrap();
        def.set(PUBLIC, public).unwrap();
        script_manager.table_defs.insert(ScriptId(1), def);

        let mut script = Script {
            script_id: ScriptId(1),
            data: ScriptData::default(),
        };

        script.load(&lua, &mut script_manager, Entity(7)).unwrap();

        assert!(matches!(
            script.data.fields.get(POSITION),
            Some(ScriptField::Vec2(v)) if *v == [12.5, -3.0]
        ));
        assert!(matches!(
            script.data.fields.get("color"),
            Some(ScriptField::Vec3(v)) if *v == [0.25, 0.5, 0.75]
        ));

        let (_, instance) = script_manager
            .instances
            .iter()
            .find(|((entity, _), _)| *entity == Entity(7))
            .expect("missing script instance");
        let public: Table = instance.get(PUBLIC).unwrap();
        let position: Table = public.get(POSITION).unwrap();
        let color: Table = public.get("color").unwrap();
        assert_eq!(position.get::<f32>("x").unwrap(), 12.5);
        assert_eq!(position.get::<f32>("y").unwrap(), -3.0);
        assert_eq!(color.get::<f32>("x").unwrap(), 0.25);
        assert_eq!(color.get::<f32>("y").unwrap(), 0.5);
        assert_eq!(color.get::<f32>("z").unwrap(), 0.75);
    }

    #[test]
    fn load_rejects_indexed_vec_tables() {
        let lua = Lua::new();
        let mut script_manager = ScriptManager::default();
        let def = lua.create_table().unwrap();
        let public = lua.create_table().unwrap();
        let position = lua.create_table().unwrap();

        position.set(1, 12.5).unwrap();
        position.set(2, -3.0).unwrap();
        public.set(POSITION, position).unwrap();
        def.set(PUBLIC, public).unwrap();
        script_manager.table_defs.insert(ScriptId(1), def);

        let mut script = Script {
            script_id: ScriptId(1),
            data: ScriptData::default(),
        };

        let error = script
            .load(&lua, &mut script_manager, Entity(7))
            .unwrap_err();

        assert!(error.to_string().contains(POSITION));
    }
}
