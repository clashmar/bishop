use engine_core::prelude::*;
use mlua::prelude::LuaResult;
use mlua::{Lua, Value};

#[derive(Clone)]
pub struct EntityHandle {
    pub entity: Entity,
}

fn entity_is_alive(ecs: &Ecs, entity: Entity) -> bool {
    COMPONENTS
        .iter()
        .any(|registry| (registry.has)(ecs, entity))
}

pub fn ensure_live_entity(ecs: &Ecs, entity: Entity) -> LuaResult<()> {
    if entity_is_alive(ecs, entity) {
        Ok(())
    } else {
        Err(mlua::Error::RuntimeError(format!(
            "Entity {} is no longer alive",
            *entity
        )))
    }
}

pub fn lua_entity_handle(lua: &Lua, entity: Entity) -> LuaResult<Value> {
    let handle = EntityHandle { entity };
    lua.create_userdata(handle).map(Value::UserData)
}
