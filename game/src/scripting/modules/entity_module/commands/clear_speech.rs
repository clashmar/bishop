use crate::game_global::push_command;
use crate::scripting::commands::text_commands::ClearSpeechCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::UserDataMethods;

pub struct ClearSpeechMethod;

impl LuaMethod<EntityHandle> for ClearSpeechMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(CLEAR_SPEECH, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            push_command(Box::new(ClearSpeechCmd {
                entity: this.entity,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Removes any speech bubble from the entity.");
        out.line(&format!("function Entity:{}() end", CLEAR_SPEECH));
        out.line("");
    }
}
