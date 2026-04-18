use crate::game_global::push_command;
use crate::scripting::commands::entity::SetComponentCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::to_snake_case;
use mlua::{UserDataMethods, Value};

pub struct SetMethod;

impl LuaMethod<EntityHandle> for SetMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(
            lua_entity::SET,
            |lua, this, (comp_name, value): (String, Value)| {
                let ctx = LuaGameCtx::borrow_ctx(lua)?;
                let game_instance = ctx.game_instance.borrow();
                ensure_live_entity(&game_instance.game.ecs, this.entity)?;
                public_lua_component(&comp_name).map_err(mlua::Error::RuntimeError)?;
                push_command(Box::new(SetComponentCmd {
                    entity: *this.entity,
                    comp_name,
                    value,
                }));
                Ok(())
            },
        );

        for reg in public_lua_components() {
            let comp_name = reg.type_name.to_string();
            let fn_name = format!("{}_{}", lua_entity::SET, to_snake_case(reg.type_name));
            methods.add_method(fn_name.as_str(), move |lua, this, value: Value| {
                let ctx = LuaGameCtx::borrow_ctx(lua)?;
                let game_instance = ctx.game_instance.borrow();
                ensure_live_entity(&game_instance.game.ecs, this.entity)?;
                push_command(Box::new(SetComponentCmd {
                    entity: *this.entity,
                    comp_name: comp_name.clone(),
                    value,
                }));
                Ok(())
            });
        }
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("-- Generic set method");
        out.line("---@param component ComponentId");
        out.line("---@param value any");
        out.line(&format!(
            "function Entity:{}(component, value) end",
            lua_entity::SET
        ));
        out.line("");

        out.line("-- Typed component setters");
        for reg in public_lua_components() {
            let type_name = reg.type_name;
            let fn_name = to_snake_case(type_name);
            out.line("---@param self Entity");
            out.line(&format!("---@param v {}", type_name));
            out.line(&format!(
                "function Entity:{}_{}(v) end",
                lua_entity::SET,
                fn_name,
            ));
            out.line("");
        }
    }
}
