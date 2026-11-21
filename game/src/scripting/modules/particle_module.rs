use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::lua_entity_handle;
use engine_core::ecs::{
    Active, ParticleConfigIssue, ParticleEmitter, ParticleEmitterConfig, Transform,
};
use engine_core::scripting::lua_constants::{lua_engine, lua_files, lua_globals, lua_particles};
use engine_core::scripting::parse_named_vec2;
use engine_core::scripting::{LuaApi, LuaApiWriter, LuaModule};
use engine_core::worlds::RoomLayer;
use engine_core::{register_lua_api, register_lua_module};
use mlua::prelude::LuaResult;
use mlua::{Lua, LuaSerdeExt, Table, Value};
use serde_json::Value as JsonValue;

/// Registers Lua creation for entity-owned particle emitters.
#[derive(Default)]
pub struct ParticleModule;
register_lua_module!(ParticleModule);
register_lua_api!(ParticleModule, lua_files::ENGINE);

impl LuaModule for ParticleModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_table: Table = lua.globals().get(lua_engine::ENGINE)?;
        let particles_table = lua.create_table()?;

        particles_table.set(
            lua_particles::CREATE,
            lua.create_function(
                |lua, (position, config): (Table, Option<Table>)| {
                    let position = parse_named_vec2(
                        &position,
                        &format!(
                            "{}.{}.{}.{}",
                            lua_engine::ENGINE,
                            lua_particles::MODULE,
                            lua_particles::CREATE,
                            lua_particles::POSITION,
                        ),
                    )?;
                    if !position.x.is_finite() || !position.y.is_finite() {
                        return Err(mlua::Error::RuntimeError(
                            "Particle emitter position must be finite".into(),
                        ));
                    }
                    let config = decode_config(lua, config)?;
                    let ctx = LuaGameCtx::borrow_ctx(lua)?;
                    let entity = {
                        let mut game_instance = ctx.game_instance.borrow_mut();
                        let room_id = game_instance
                            .game
                            .current_world()
                            .current_room()
                            .map(|room| room.id)
                            .ok_or_else(|| {
                                mlua::Error::RuntimeError(format!(
                                    "{}.{}.{} requires an active current room",
                                    lua_engine::ENGINE,
                                    lua_particles::MODULE,
                                    lua_particles::CREATE,
                                ))
                            })?;
                        game_instance
                            .game
                            .ecs
                            .create_entity()
                            .with(Active::default())
                            .with(Transform {
                                position,
                                ..Transform::default()
                            })
                            .with(ParticleEmitter { config })
                            .with_current_room_layer(room_id, RoomLayer::Front)
                            .finish()
                    };
                    lua_entity_handle(lua, entity)
                },
            )?,
        )?;

        engine_table.set(lua_particles::MODULE, particles_table)?;
        Ok(())
    }
}

impl LuaApi for ParticleModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(&format!("---@class {}", lua_particles::CLASS_FRAME));
        out.line(&format!("---@field {} string", lua_particles::CLIP_ID));
        out.line(&format!("---@field {} integer", lua_particles::COL));
        out.line(&format!("---@field {} integer", lua_particles::ROW));
        out.line(&format!("---@field {} vec2", lua_particles::OFFSET));
        out.line(&format!("---@field {} integer", lua_particles::SPRITE_ID));
        out.line(&format!("---@field {} vec2", lua_particles::FRAME_SIZE));
        out.line(&format!("---@field {} boolean", lua_particles::FLIP_X));
        out.line("");
        out.line(&format!("---@class {}", lua_particles::CLASS_SPRITE_VISUAL));
        out.line(&format!(
            "---@field {} {}",
            lua_particles::VISUAL_SPRITE,
            lua_particles::CLASS_SPRITE_VISUAL_DATA,
        ));
        out.line("");
        out.line(&format!("---@class {}", lua_particles::CLASS_SPRITE_VISUAL_DATA));
        out.line(&format!(
            "---@field {} {}",
            lua_particles::FRAME,
            lua_particles::CLASS_FRAME,
        ));
        out.line("");
        out.line(&format!("---@class {}", lua_particles::CLASS_CONFIG));
        out.line(&format!(
            "---@field {}? '{}'|'{}'",
            lua_particles::MODE,
            lua_particles::MODE_BURST,
            lua_particles::MODE_CONTINUOUS,
        ));
        out.line(&format!(
            "---@field {}? '{}'|'{}'|{}",
            lua_particles::VISUAL,
            lua_particles::VISUAL_PIXEL,
            lua_particles::VISUAL_LINE,
            lua_particles::CLASS_SPRITE_VISUAL,
        ));
        out.line(&format!("---@field {}? vec2", lua_particles::AREA_SIZE));
        out.line(&format!("---@field {}? integer", lua_particles::LINE_LENGTH));
        out.line(&format!("---@field {}? number", lua_particles::LIFETIME_MIN));
        out.line(&format!("---@field {}? number", lua_particles::LIFETIME_MAX));
        out.line(&format!("---@field {}? vec2", lua_particles::VELOCITY_MIN));
        out.line(&format!("---@field {}? vec2", lua_particles::VELOCITY_MAX));
        out.line(&format!("---@field {}? number", lua_particles::SCALE_START));
        out.line(&format!("---@field {}? number", lua_particles::SCALE_END));
        out.line(&format!("---@field {}? Color", lua_particles::COLOR));
        out.line(&format!("---@field {}? number", lua_particles::OPACITY_START));
        out.line(&format!("---@field {}? number", lua_particles::OPACITY_END));
        out.line(&format!("---@field {}? integer", lua_particles::BURST_COUNT));
        out.line(&format!("---@field {}? number", lua_particles::EMISSION_RATE));
        out.line(&format!("---@field {}? integer", lua_particles::MAX_PARTICLES));
        out.line(&format!("---@field {}? integer", lua_particles::SEED));
        out.line("");
        out.line(&format!("{}.{} = {{}}", lua_engine::ENGINE, lua_particles::MODULE));
        out.line("");
        out.line(&format!("---@param {} vec2", lua_particles::POSITION));
        out.line(&format!(
            "---@param {}? {}",
            lua_particles::CONFIG,
            lua_particles::CLASS_CONFIG,
        ));
        out.line(&format!("---@return {}", lua_globals::ENTITY));
        out.line(&format!(
            "function {}.{}.{}({}, {}) end",
            lua_engine::ENGINE,
            lua_particles::MODULE,
            lua_particles::CREATE,
            lua_particles::POSITION,
            lua_particles::CONFIG,
        ));
        out.line("");
    }
}

pub(super) fn decode_config(lua: &Lua, config: Option<Table>) -> LuaResult<ParticleEmitterConfig> {
    let Some(config) = config else {
        return Ok(ParticleEmitterConfig::default());
    };
    let mut normalized: JsonValue = lua.from_value(Value::Table(config.clone()))?;
    normalize_named_vec2(
        &config,
        &mut normalized,
        lua_particles::AREA_SIZE,
        "particle emitter area_size",
    )?;
    normalize_named_vec2(
        &config,
        &mut normalized,
        lua_particles::VELOCITY_MIN,
        "particle emitter velocity_min",
    )?;
    normalize_named_vec2(
        &config,
        &mut normalized,
        lua_particles::VELOCITY_MAX,
        "particle emitter velocity_max",
    )?;

    if let Value::Table(visual) = config.get::<Value>(lua_particles::VISUAL)? {
        if let Some(sprite) = visual.get::<Option<Table>>(lua_particles::VISUAL_SPRITE)? {
            if let Some(frame) = sprite.get::<Option<Table>>(lua_particles::FRAME)? {
                let normalized_frame = normalized
                    .get_mut(lua_particles::VISUAL)
                    .and_then(|value| value.get_mut(lua_particles::VISUAL_SPRITE))
                    .and_then(|value| value.get_mut(lua_particles::FRAME))
                    .ok_or_else(|| {
                        mlua::Error::RuntimeError(
                            "Invalid particle sprite frame configuration".into(),
                        )
                    })?;
                normalize_named_vec2(
                    &frame,
                    normalized_frame,
                    lua_particles::OFFSET,
                    "particle sprite frame offset",
                )?;
                normalize_named_vec2(
                    &frame,
                    normalized_frame,
                    lua_particles::FRAME_SIZE,
                    "particle sprite frame frame_size",
                )?;
            }
        }
    }

    let config = serde_json::from_value::<ParticleEmitterConfig>(normalized)
        .map_err(mlua::Error::external)?;
    config.validate().map_err(config_error)?;
    Ok(config)
}

fn config_error(issues: Vec<ParticleConfigIssue>) -> mlua::Error {
    mlua::Error::RuntimeError(format_particle_config_issues(&issues))
}

pub(crate) fn format_particle_config_issues(issues: &[ParticleConfigIssue]) -> String {
    let messages = issues
        .iter()
        .map(|issue| issue.message())
        .collect::<Vec<_>>()
        .join("; ");
    format!("Invalid particle emitter configuration: {messages}")
}

fn normalize_named_vec2(
    source: &Table,
    target: &mut JsonValue,
    field: &str,
    context: &str,
) -> LuaResult<()> {
    let Some(vector) = source.get::<Option<Table>>(field)? else {
        return Ok(());
    };
    if vector.raw_len() > 0 {
        return Ok(());
    }
    let vector = parse_named_vec2(&vector, context)?;
    let value = serde_json::json!([vector.x, vector.y]);
    let Some(normalized) = target.get_mut(field) else {
        return Err(mlua::Error::RuntimeError(format!(
            "Missing normalized field '{field}'"
        )));
    };
    *normalized = value;
    Ok(())
}
