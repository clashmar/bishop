use crate::game_global::push_command;
use crate::scripting::commands::particle_commands::{
    ParticleEmitterAction, ParticleEmitterCommand,
};
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use crate::scripting::modules::particle_module::decode_config;
use engine_core::ecs::{Ecs, Entity, ParticleEmitter};
use engine_core::scripting::lua_constants::{lua_globals, lua_particles};
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use mlua::prelude::LuaResult;
use mlua::{Lua, Table, UserDataMethods};

pub struct ConfigureParticlesMethod;
pub struct BurstParticlesMethod;
pub struct StartParticlesMethod;
pub struct StopParticlesMethod;

impl LuaMethod<EntityHandle> for ConfigureParticlesMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_particles::ENTITY_CONFIGURE, |lua, this, config: Table| {
            let config = decode_config(lua, Some(config))?;
            queue_particle_command(lua, this, ParticleEmitterAction::Configure(config))
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Replaces this entity's particle emitter settings.");
        out.line(&format!(
            "---@param {} {}",
            lua_particles::CONFIG,
            lua_particles::CLASS_CONFIG,
        ));
        out.line("---@return nil");
        out.line(&format!(
            "function {}:{}({}) end",
            lua_globals::ENTITY,
            lua_particles::ENTITY_CONFIGURE,
            lua_particles::CONFIG,
        ));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for BurstParticlesMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_particles::ENTITY_BURST, |lua, this, ()| {
            queue_particle_command(lua, this, ParticleEmitterAction::Burst)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        emit_void_method(
            out,
            lua_particles::ENTITY_BURST,
            "Emits a burst when this entity uses burst mode.",
        );
    }
}

impl LuaMethod<EntityHandle> for StartParticlesMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_particles::ENTITY_START, |lua, this, ()| {
            queue_particle_command(lua, this, ParticleEmitterAction::Start)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        emit_void_method(
            out,
            lua_particles::ENTITY_START,
            "Starts continuous emission when this entity uses continuous mode.",
        );
    }
}

impl LuaMethod<EntityHandle> for StopParticlesMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_particles::ENTITY_STOP, |lua, this, ()| {
            queue_particle_command(lua, this, ParticleEmitterAction::Stop)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        emit_void_method(
            out,
            lua_particles::ENTITY_STOP,
            "Stops continuous emission without removing live particles.",
        );
    }
}

fn queue_particle_command(
    lua: &Lua,
    this: &EntityHandle,
    action: ParticleEmitterAction,
) -> LuaResult<()> {
    {
        let ctx = LuaGameCtx::borrow_ctx(lua)?;
        let game_instance = ctx.game_instance.borrow();
        ensure_particle_emitter(&game_instance.game.ecs, this.entity)?;
    }
    push_command(Box::new(ParticleEmitterCommand {
        owner: this.entity,
        action: Some(action),
    }));
    Ok(())
}

fn ensure_particle_emitter(ecs: &Ecs, owner: Entity) -> LuaResult<()> {
    ensure_live_entity(ecs, owner)?;
    if ecs.get::<ParticleEmitter>(owner).is_none() {
        return Err(mlua::Error::RuntimeError(format!(
            "Entity {} has no ParticleEmitter component",
            *owner,
        )));
    }
    Ok(())
}

fn emit_void_method(out: &mut LuaApiWriter, method: &str, description: &str) {
    out.line(&format!("--- {description}"));
    out.line("---@return nil");
    out.line(&format!("function {}:{}() end", lua_globals::ENTITY, method));
    out.line("");
}
