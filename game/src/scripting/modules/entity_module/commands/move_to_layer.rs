use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use engine_core::worlds::RoomLayer;
use mlua::UserDataMethods;

pub struct MoveToLayerMethod;

impl LuaMethod<EntityHandle> for MoveToLayerMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::MOVE_TO_LAYER, |lua, this, layer_name: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let mut game_instance = ctx.game_instance.borrow_mut();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;

            let Some(layer) = RoomLayer::from_script_name(&layer_name) else {
                return Err(mlua::Error::RuntimeError(format!(
                    "Unknown room layer '{layer_name}'"
                )));
            };

            game_instance.game.ecs.set_entity_layer(this.entity, layer);
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Moves this entity between the Front and Back layers in its current room.");
        out.line("---@param layer string");
        out.line("---@return nil");
        out.line(&format!(
            "function Entity:{}(layer) end",
            lua_entity::MOVE_TO_LAYER
        ));
        out.line("");
    }
}
