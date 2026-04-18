use crate::game_global::push_command;
use crate::scripting::commands::entity::ResetClipCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::UserDataMethods;

pub struct ResetClipMethod;

impl LuaMethod<EntityHandle> for ResetClipMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(RESET_CLIP, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            push_command(Box::new(ResetClipCmd {
                entity: this.entity,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Resets the current clip to frame 0.");
        out.line(&format!("function Entity:{}() end", RESET_CLIP));
        out.line("");
    }
}
