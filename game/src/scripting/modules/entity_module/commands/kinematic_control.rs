use crate::game_global::push_command;
use crate::scripting::commands::entity::{KinematicCommandKind, SetKinematicCmd};
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::ecs::{Ecs, Entity, Kinematic, KinematicAxis, KinematicDirection, KinematicMotionMode};
use engine_core::scripting::lua_constants::{lua_entity, lua_kinematic};
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use mlua::UserDataMethods;

pub struct StartKinematicMethod;
pub struct StopKinematicMethod;
pub struct ReverseKinematicMethod;
pub struct SetKinematicEnabledMethod;
pub struct SetKinematicModeMethod;
pub struct SetKinematicAxisMethod;
pub struct SetKinematicDirectionMethod;
pub struct SetKinematicSpeedMethod;
pub struct SetKinematicTravelDistanceMethod;

impl LuaMethod<EntityHandle> for StartKinematicMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::START_KINEMATIC, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            require_kinematic(ecs, this.entity)?;
            push_command(Box::new(SetKinematicCmd {
                entity: this.entity,
                kind: KinematicCommandKind::Start,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@return nil");
        out.line(&format!("function Entity:{}() end", lua_entity::START_KINEMATIC));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for StopKinematicMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::STOP_KINEMATIC, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            require_kinematic(ecs, this.entity)?;
            push_command(Box::new(SetKinematicCmd {
                entity: this.entity,
                kind: KinematicCommandKind::Stop,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@return nil");
        out.line(&format!("function Entity:{}() end", lua_entity::STOP_KINEMATIC));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for ReverseKinematicMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::REVERSE_KINEMATIC, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            let kinematic = require_kinematic(ecs, this.entity)?;
            if kinematic.motion.mode != KinematicMotionMode::PingPong {
                return Err(mlua::Error::RuntimeError(
                    "reverse_kinematic requires ping_pong motion".into(),
                ));
            }
            push_command(Box::new(SetKinematicCmd {
                entity: this.entity,
                kind: KinematicCommandKind::Reverse,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@return nil");
        out.line(&format!("function Entity:{}() end", lua_entity::REVERSE_KINEMATIC));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for SetKinematicEnabledMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::SET_KINEMATIC_ENABLED, |lua, this, enabled: bool| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            require_kinematic(ecs, this.entity)?;
            push_command(Box::new(SetKinematicCmd {
                entity: this.entity,
                kind: KinematicCommandKind::SetEnabled(enabled),
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@param enabled boolean");
        out.line("---@return nil");
        out.line(&format!("function Entity:{}(enabled) end", lua_entity::SET_KINEMATIC_ENABLED));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for SetKinematicModeMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::SET_KINEMATIC_MODE, |lua, this, mode: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            require_kinematic(ecs, this.entity)?;
            let mode = parse_kinematic_value::<KinematicMotionMode>(&mode, "mode")?;
            push_command(Box::new(SetKinematicCmd {
                entity: this.entity,
                kind: KinematicCommandKind::SetMode(mode),
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(&format!(
            "---@param mode '{}'|'{}'|'{}'",
            lua_kinematic::MODE_NONE,
            lua_kinematic::MODE_CONSTANT,
            lua_kinematic::MODE_PING_PONG,
        ));
        out.line("---@return nil");
        out.line(&format!("function Entity:{}(mode) end", lua_entity::SET_KINEMATIC_MODE));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for SetKinematicAxisMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::SET_KINEMATIC_AXIS, |lua, this, axis: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            require_kinematic(ecs, this.entity)?;
            let axis = parse_kinematic_value::<KinematicAxis>(&axis, "axis")?;
            push_command(Box::new(SetKinematicCmd {
                entity: this.entity,
                kind: KinematicCommandKind::SetAxis(axis),
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(&format!(
            "---@param axis '{}'|'{}'",
            lua_kinematic::AXIS_HORIZONTAL,
            lua_kinematic::AXIS_VERTICAL,
        ));
        out.line("---@return nil");
        out.line(&format!("function Entity:{}(axis) end", lua_entity::SET_KINEMATIC_AXIS));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for SetKinematicDirectionMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::SET_KINEMATIC_DIRECTION, |lua, this, direction: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            require_kinematic(ecs, this.entity)?;
            let direction = parse_kinematic_value::<KinematicDirection>(&direction, "direction")?;
            push_command(Box::new(SetKinematicCmd {
                entity: this.entity,
                kind: KinematicCommandKind::SetDirection(direction),
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(&format!(
            "---@param direction '{}'|'{}'",
            lua_kinematic::DIRECTION_POSITIVE,
            lua_kinematic::DIRECTION_NEGATIVE,
        ));
        out.line("---@return nil");
        out.line(&format!("function Entity:{}(direction) end", lua_entity::SET_KINEMATIC_DIRECTION));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for SetKinematicSpeedMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::SET_KINEMATIC_SPEED, |lua, this, speed: f32| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            require_kinematic(ecs, this.entity)?;
            if speed < 0.0 {
                return Err(mlua::Error::RuntimeError(
                    "Kinematic speed must be non-negative".into(),
                ));
            }
            push_command(Box::new(SetKinematicCmd {
                entity: this.entity,
                kind: KinematicCommandKind::SetSpeed(speed),
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@param speed number");
        out.line("---@return nil");
        out.line(&format!("function Entity:{}(speed) end", lua_entity::SET_KINEMATIC_SPEED));
        out.line("");
    }
}

impl LuaMethod<EntityHandle> for SetKinematicTravelDistanceMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::SET_KINEMATIC_TRAVEL_DISTANCE, |lua, this, distance: f32| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            require_kinematic(ecs, this.entity)?;
            if distance < 0.0 {
                return Err(mlua::Error::RuntimeError(
                    "Kinematic travel distance must be non-negative".into(),
                ));
            }
            push_command(Box::new(SetKinematicCmd {
                entity: this.entity,
                kind: KinematicCommandKind::SetTravelDistance(distance),
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@param distance number");
        out.line("---@return nil");
        out.line(&format!("function Entity:{}(distance) end", lua_entity::SET_KINEMATIC_TRAVEL_DISTANCE));
        out.line("");
    }
}

fn require_kinematic(ecs: &Ecs, entity: Entity) -> mlua::Result<&Kinematic> {
    ecs.get::<Kinematic>(entity).ok_or_else(|| {
        mlua::Error::RuntimeError(format!("Entity {} does not have Kinematic", *entity))
    })
}

fn parse_kinematic_value<T>(value: &str, kind: &str) -> mlua::Result<T>
where
    T: std::str::FromStr,
{
    value.parse::<T>().map_err(|_| {
        mlua::Error::RuntimeError(format!("Invalid kinematic {kind} '{value}'"))
    })
}
