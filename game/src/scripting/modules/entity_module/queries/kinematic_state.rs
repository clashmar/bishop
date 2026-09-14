use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::ecs::{Ecs, Entity, Kinematic};
use engine_core::scripting::lua_constants::{lua_entity, lua_kinematic};
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use mlua::UserDataMethods;
use strum::EnumProperty;

const LUA_NAME_PROPERTY: &str = "lua";

pub struct IsKinematicRunningMethod;
pub struct GetKinematicStateMethod;

impl LuaMethod<EntityHandle> for IsKinematicRunningMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::IS_KINEMATIC_RUNNING, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            Ok(require_kinematic(ecs, this.entity)?.is_runtime_running())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}() end", lua_entity::IS_KINEMATIC_RUNNING));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for GetKinematicStateMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::GET_KINEMATIC_STATE, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            let kinematic = require_kinematic(ecs, this.entity)?;
            let table = lua.create_table()?;
            table.set(lua_kinematic::STATE_ENABLED, kinematic.is_runtime_enabled())?;
            table.set(lua_kinematic::STATE_RUNNING, kinematic.is_runtime_running())?;
            table.set(
                lua_kinematic::STATE_MODE,
                kinematic_lua_name(&kinematic.motion.mode),
            )?;
            table.set(
                lua_kinematic::STATE_AXIS,
                kinematic_lua_name(&kinematic.motion.axis),
            )?;
            table.set(
                lua_kinematic::STATE_DIRECTION,
                kinematic_lua_name(&kinematic.runtime_direction()),
            )?;
            table.set(lua_kinematic::STATE_SPEED, kinematic.motion.speed)?;
            table.set(
                lua_kinematic::STATE_TRAVEL_DISTANCE,
                kinematic.motion.travel_distance,
            )?;
            Ok(table)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(&format!(
            "---@return {{ {}: boolean, {}: boolean, {}: string, {}: string, {}: string, {}: number, {}: number }}",
            lua_kinematic::STATE_ENABLED,
            lua_kinematic::STATE_RUNNING,
            lua_kinematic::STATE_MODE,
            lua_kinematic::STATE_AXIS,
            lua_kinematic::STATE_DIRECTION,
            lua_kinematic::STATE_SPEED,
            lua_kinematic::STATE_TRAVEL_DISTANCE,
        ));
        out.line(&format!("function Entity:{}() end", lua_entity::GET_KINEMATIC_STATE));
        out.line("");
    }
}

fn require_kinematic(ecs: &Ecs, entity: Entity) -> mlua::Result<&Kinematic> {
    ecs.get::<Kinematic>(entity).ok_or_else(|| {
        mlua::Error::RuntimeError(format!("Entity {} does not have Kinematic", *entity))
    })
}

fn kinematic_lua_name<T>(value: &T) -> &'static str
where
    T: EnumProperty,
{
    let name = value.get_str(LUA_NAME_PROPERTY);
    debug_assert!(name.is_some(), "missing Lua name on kinematic enum");
    name.unwrap_or("")
}
