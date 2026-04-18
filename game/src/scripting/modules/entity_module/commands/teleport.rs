use crate::game_global::push_command;
use crate::scripting::commands::entity::RepositionEntityCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::TELEPORT;
use engine_core::scripting::parse_named_vec2;
use mlua::{Table, UserDataMethods};

pub struct TeleportMethod;

impl LuaMethod<EntityHandle> for TeleportMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(TELEPORT, |lua, this, position: Table| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            let target_position = parse_named_vec2(&position, &format!("Entity:{TELEPORT} position"))?;
            push_command(Box::new(RepositionEntityCmd {
                entity: this.entity,
                target_position,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Instantly moves the entity to an absolute world position.");
        out.line("---@param position vec2");
        out.line(&format!("function Entity:{}(position) end", TELEPORT));
        out.line("");
    }
}
