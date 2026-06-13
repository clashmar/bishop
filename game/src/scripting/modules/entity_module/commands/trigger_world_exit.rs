use crate::game_global::set_pending_world_transition;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use crate::transitions::world_transitions::{WorldSelector, WorldTransitionRequest};
use engine_core::ecs::WorldExit;
use engine_core::omni_error;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use engine_core::worlds::WorldTransitionMode;
use mlua::UserDataMethods;

/// Lua method executing the entity's `WorldExit` component.
pub struct TriggerWorldExitMethod;

impl LuaMethod<EntityHandle> for TriggerWorldExitMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::TRIGGER_WORLD_EXIT, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;

            let Some(world_exit) = game_instance.game.ecs.get::<WorldExit>(this.entity) else {
                omni_error!("trigger_world_exit on entity without a WorldExit component");
                return Ok(());
            };

            let Some(dest) = world_exit.destination_world else {
                omni_error!("trigger_world_exit called on unconfigured WorldExit (no destination set)");
                return Ok(());
            };

            // Transport moves the player; Activate moves no entity.
            let entity = match world_exit.mode {
                WorldTransitionMode::Transport => game_instance.game.ecs.get_player_entity(),
                WorldTransitionMode::Activate => None,
            };

            set_pending_world_transition(WorldTransitionRequest {
                entity,
                world: WorldSelector::ById(dest),
                entry_name: world_exit.entry.clone(),
                mode: world_exit.mode,
            });
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Executes this entity's WorldExit component (transport the player or activate a world).");
        out.line(&format!("function Entity:{}() end", lua_entity::TRIGGER_WORLD_EXIT));
        out.line("");
    }
}
