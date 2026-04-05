// engine_core/src/script/script.rs
use crate::ecs::entity::Entity;
use crate::game::EngineCtxMut;
use crate::scripting::lua_constants::PUBLIC;
use crate::scripting::script_manager::ScriptManager;
use ecs_component::ecs_component;
use mlua::Lua;
use mlua::Table;
use mlua::Value;
use mlua::prelude::LuaResult;
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
            let field = match value {
                Value::Boolean(b) => ScriptField::Bool(b),
                Value::Integer(i) => ScriptField::Int(i),
                Value::Number(n) => ScriptField::Float(n),
                Value::String(s) => ScriptField::Text(s.to_str()?.to_string()),
                Value::Table(t) => {
                    // Try Vec2
                    if let Ok(x) = t.get::<f64>(1) {
                        if let Ok(y) = t.get::<f64>(2) {
                            if let Ok(z) = t.get::<f64>(3) {
                                ScriptField::Vec3([x as f32, y as f32, z as f32])
                            } else {
                                ScriptField::Vec2([x as f32, y as f32])
                            }
                        } else {
                            // Skip unsupported table
                            continue;
                        }
                    } else {
                        // Skip unsupported table
                        continue;
                    }
                }
                // Ignore functions
                _ => continue,
            };
            fields.insert(name, field);
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
            match field {
                ScriptField::Bool(b) => public.set(name.clone(), *b)?,
                ScriptField::Int(i) => public.set(name.clone(), *i)?,
                ScriptField::Float(f) => public.set(name.clone(), *f)?,
                ScriptField::Text(s) => public.set(name.clone(), s.clone())?,
                ScriptField::Vec2(v) => {
                    let t = lua.create_table()?;
                    t.set(1, v[0])?;
                    t.set(2, v[1])?;
                    public.set(name.clone(), t)?;
                }
                ScriptField::Vec3(v) => {
                    let t = lua.create_table()?;
                    t.set(1, v[0])?;
                    t.set(2, v[1])?;
                    t.set(3, v[2])?;
                    public.set(name.clone(), t)?;
                }
            }
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
