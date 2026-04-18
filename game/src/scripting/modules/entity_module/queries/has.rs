use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::prelude::LuaResult;
use mlua::{UserDataMethods, Variadic};

pub struct HasMethod;

impl LuaMethod<EntityHandle> for HasMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(HAS, |lua, this, comp_name: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            let reg = public_lua_component(&comp_name).map_err(mlua::Error::RuntimeError)?;
            Ok((reg.has)(ecs, this.entity))
        });

        methods.add_method(HAS_ANY, |lua, this, comps: Variadic<String>| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            let regs = comps
                .iter()
                .map(|comp_name| public_lua_component(comp_name).map_err(mlua::Error::RuntimeError))
                .collect::<LuaResult<Vec<_>>>()?;
            for reg in regs {
                if (reg.has)(ecs, this.entity) {
                    return Ok(true);
                }
            }
            Ok(false)
        });

        methods.add_method(HAS_ALL, |lua, this, comps: Variadic<String>| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            let regs = comps
                .iter()
                .map(|comp_name| public_lua_component(comp_name).map_err(mlua::Error::RuntimeError))
                .collect::<LuaResult<Vec<_>>>()?;
            for reg in regs {
                if !(reg.has)(ecs, this.entity) {
                    return Ok(false);
                }
            }
            Ok(true)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@param component ComponentId");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}(component) end", HAS));
        out.line("");

        out.line("---@param ... ComponentId");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}(...) end", HAS_ANY));
        out.line("");

        out.line("---@param ... ComponentId");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}(...) end", HAS_ALL));
        out.line("");
    }
}
