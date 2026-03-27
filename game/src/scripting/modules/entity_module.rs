// game/src/scripting/modules/entity_module.rs
use crate::game_global::push_command;
use crate::scripting::commands::lua_command::*;
use crate::scripting::commands::text_commands::*;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::lua_helpers::*;
use engine_core::prelude::*;
use mlua::prelude::LuaResult;
use mlua::Lua;
use mlua::Table;
use mlua::UserData;
use mlua::UserDataMethods;
use mlua::UserDataRegistry;
use mlua::Value;
use mlua::Variadic;
use std::collections::HashMap;

/// Lua module that exposes a constructor for `EntityHandle`.
#[derive(Default)]
pub struct EntityModule;
register_lua_module!(EntityModule);

impl LuaModule for EntityModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        // Wraps an entity(id) in a lua EntityHandle
        let factory =
            lua.create_function(|_, id: usize| Ok(EntityHandle { entity: Entity(id) }))?;
        lua.globals().set(ENTITY, factory)?;
        Ok(())
    }
}

register_lua_api!(EntityModule, ENTITY_FILE);

impl LuaApi for EntityModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        // Define entity class
        out.line("---@class Entity");
        out.line("---@field id integer");
        out.line("local Entity = {}");
        out.line("");

        // Emit each registered method
        for m in entity_handle_methods().iter() {
            m.emit_api(out);
        }

        out.line("return Entity");
    }
}

/// A lua wrapper that carries an Entity id.
#[derive(Clone)]
pub struct EntityHandle {
    pub entity: Entity,
}

/// Build a Lua userdata object that wraps `Entity`.
pub fn lua_entity_handle(lua: &Lua, entity: Entity) -> LuaResult<Value> {
    let handle = EntityHandle { entity };
    lua.create_userdata(handle).map(Value::UserData)
}

impl UserData for EntityHandle {
    fn add_methods<'lua, M: UserDataMethods<Self>>(methods: &mut M) {
        for m in &entity_handle_methods() {
            m.register(methods);
        }
    }

    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get(ID, |_, this| Ok(*this.entity));
    }

    fn register(registry: &mut UserDataRegistry<Self>) {
        Self::add_fields(registry);
        Self::add_methods(registry);
    }
}

pub enum EntityHandleMethod {
    Get(GetMethod),
    Set(SetMethod),
    Has(HasMethod),
    Interact(InteractMethod),
    FindBestInteractable(FindBestInteractableMethod),
    SetClip(SetClipMethod),
    GetClip(GetClipMethod),
    ResetClip(ResetClipMethod),
    SetFlipX(SetFlipXMethod),
    GetFlipX(GetFlipXMethod),
    SetFacing(SetFacingMethod),
    SetAnimSpeed(SetAnimSpeedMethod),
    GetCurrentFrame(GetCurrentFrameMethod),
    IsClipFinished(IsClipFinishedMethod),
    Say(SayMethod),
    ClearSpeech(ClearSpeechMethod),
    IsSpeaking(IsSpeakingMethod),
    PlaySound(PlaySoundMethod),
    StopSound(StopSoundMethod),
    SetSoundVolume(SetSoundVolumeMethod),
}

/// Returns all entity handle methods.
fn entity_handle_methods() -> Vec<EntityHandleMethod> {
    vec![
        EntityHandleMethod::Get(GetMethod),
        EntityHandleMethod::Set(SetMethod),
        EntityHandleMethod::Has(HasMethod),
        EntityHandleMethod::Interact(InteractMethod),
        EntityHandleMethod::FindBestInteractable(FindBestInteractableMethod),
        EntityHandleMethod::SetClip(SetClipMethod),
        EntityHandleMethod::GetClip(GetClipMethod),
        EntityHandleMethod::ResetClip(ResetClipMethod),
        EntityHandleMethod::SetFlipX(SetFlipXMethod),
        EntityHandleMethod::GetFlipX(GetFlipXMethod),
        EntityHandleMethod::SetFacing(SetFacingMethod),
        EntityHandleMethod::SetAnimSpeed(SetAnimSpeedMethod),
        EntityHandleMethod::GetCurrentFrame(GetCurrentFrameMethod),
        EntityHandleMethod::IsClipFinished(IsClipFinishedMethod),
        EntityHandleMethod::Say(SayMethod),
        EntityHandleMethod::ClearSpeech(ClearSpeechMethod),
        EntityHandleMethod::IsSpeaking(IsSpeakingMethod),
        EntityHandleMethod::PlaySound(PlaySoundMethod),
        EntityHandleMethod::StopSound(StopSoundMethod),
        EntityHandleMethod::SetSoundVolume(SetSoundVolumeMethod),
    ]
}

impl LuaMethod<EntityHandle> for EntityHandleMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        match self {
            EntityHandleMethod::Get(m) => m.register(methods),
            EntityHandleMethod::Set(m) => m.register(methods),
            EntityHandleMethod::Has(m) => m.register(methods),
            EntityHandleMethod::Interact(m) => m.register(methods),
            EntityHandleMethod::FindBestInteractable(m) => m.register(methods),
            EntityHandleMethod::SetClip(m) => m.register(methods),
            EntityHandleMethod::GetClip(m) => m.register(methods),
            EntityHandleMethod::ResetClip(m) => m.register(methods),
            EntityHandleMethod::SetFlipX(m) => m.register(methods),
            EntityHandleMethod::GetFlipX(m) => m.register(methods),
            EntityHandleMethod::SetFacing(m) => m.register(methods),
            EntityHandleMethod::SetAnimSpeed(m) => m.register(methods),
            EntityHandleMethod::GetCurrentFrame(m) => m.register(methods),
            EntityHandleMethod::IsClipFinished(m) => m.register(methods),
            EntityHandleMethod::Say(m) => m.register(methods),
            EntityHandleMethod::ClearSpeech(m) => m.register(methods),
            EntityHandleMethod::IsSpeaking(m) => m.register(methods),
            EntityHandleMethod::PlaySound(m) => m.register(methods),
            EntityHandleMethod::StopSound(m) => m.register(methods),
            EntityHandleMethod::SetSoundVolume(m) => m.register(methods),
        }
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        match self {
            EntityHandleMethod::Get(m) => m.emit_api(out),
            EntityHandleMethod::Set(m) => m.emit_api(out),
            EntityHandleMethod::Has(m) => m.emit_api(out),
            EntityHandleMethod::Interact(m) => m.emit_api(out),
            EntityHandleMethod::FindBestInteractable(m) => m.emit_api(out),
            EntityHandleMethod::SetClip(m) => m.emit_api(out),
            EntityHandleMethod::GetClip(m) => m.emit_api(out),
            EntityHandleMethod::ResetClip(m) => m.emit_api(out),
            EntityHandleMethod::SetFlipX(m) => m.emit_api(out),
            EntityHandleMethod::GetFlipX(m) => m.emit_api(out),
            EntityHandleMethod::SetFacing(m) => m.emit_api(out),
            EntityHandleMethod::SetAnimSpeed(m) => m.emit_api(out),
            EntityHandleMethod::GetCurrentFrame(m) => m.emit_api(out),
            EntityHandleMethod::IsClipFinished(m) => m.emit_api(out),
            EntityHandleMethod::Say(m) => m.emit_api(out),
            EntityHandleMethod::ClearSpeech(m) => m.emit_api(out),
            EntityHandleMethod::IsSpeaking(m) => m.emit_api(out),
            EntityHandleMethod::PlaySound(m) => m.emit_api(out),
            EntityHandleMethod::StopSound(m) => m.emit_api(out),
            EntityHandleMethod::SetSoundVolume(m) => m.emit_api(out),
        }
    }
}

/// Method: `entity:get("Component")`
pub struct GetMethod;
impl LuaMethod<EntityHandle> for GetMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(GET, |lua, this, comp_name: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let entity = this.entity;

            if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == comp_name) {
                if (reg.has)(ecs, entity) {
                    let boxed = (reg.clone)(ecs, entity);
                    (reg.to_lua)(lua, &*boxed)
                } else {
                    Err(mlua::Error::RuntimeError(format!(
                        "Entity {:?} has no {} component",
                        entity, comp_name
                    )))
                }
            } else {
                Err(mlua::Error::RuntimeError(format!(
                    "Component '{}' not known",
                    comp_name
                )))
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("-- Component getters");
        for reg in COMPONENTS.iter() {
            out.line(&format!(
                "---@overload fun(self: Entity, component: \"{}\"): {}",
                reg.type_name, reg.type_name
            ));
        }
        out.line("---@param component string");
        out.line("---@return table|nil");
        out.line(&format!("function Entity:{}(component) end", GET));
        out.line("");
    }
}

/// Method: `entity:set("Component", value)`
pub struct SetMethod;
impl LuaMethod<EntityHandle> for SetMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(SET, |_lua, this, (comp_name, value): (String, Value)| {
            push_command(Box::new(SetComponentCmd {
                entity: *this.entity,
                comp_name,
                value,
            }));
            Ok(())
        });

        // Typed setters
        for reg in COMPONENTS.iter() {
            let comp_name = reg.type_name.to_string();
            let fn_name = format!("{}_{}", SET, to_snake_case(reg.type_name));
            methods.add_method(fn_name.as_str(), move |_lua, this, value: Value| {
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
        out.line("---@param component string");
        out.line("---@see ComponentId");
        out.line("---@param value table");
        out.line(&format!("function Entity:{}(component, value) end", SET));
        out.line("");

        out.line("-- Typed component setters");
        for reg in COMPONENTS.iter() {
            let type_name = reg.type_name;
            let fn_name = to_snake_case(type_name);
            out.line("---@param self Entity");
            out.line(&format!("---@param v {}", type_name));
            out.line(&format!("function Entity:{}_{}(v) end", SET, fn_name));
            out.line("");
        }
    }
}

/// Method: `entity:has(...)`, `has_any`, `has_all`
pub struct HasMethod;
impl LuaMethod<EntityHandle> for HasMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        // entity:has
        methods.add_method(HAS, |lua, this, comp_name: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            Ok(COMPONENTS
                .iter()
                .find(|r| r.type_name == comp_name)
                .is_some_and(|r| (r.has)(ecs, this.entity)))
        });

        // entity:has_any
        methods.add_method(HAS_ANY, |lua, this, comps: Variadic<String>| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            for comp_name in comps.iter() {
                if let Some(r) = COMPONENTS.iter().find(|r| r.type_name == comp_name) {
                    if (r.has)(ecs, this.entity) {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        });

        // entity:has_all
        methods.add_method(HAS_ALL, |lua, this, comps: Variadic<String>| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            for comp_name in comps.iter() {
                if let Some(r) = COMPONENTS.iter().find(|r| r.type_name == comp_name) {
                    if !(r.has)(ecs, this.entity) {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }
            Ok(true)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        // has
        out.line("---@param component string");
        out.line("---@see ComponentId");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}(component) end", HAS));
        out.line("");

        // has_any
        out.line("---@param ... string");
        out.line("---@see ComponentId");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}(...) end", HAS_ANY));
        out.line("");

        // has_all
        out.line("---@param ... string");
        out.line("---@see ComponentId");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}(...) end", HAS_ALL));
        out.line("");
    }
}

/// Method: `entity:interact()`
pub struct InteractMethod;
impl LuaMethod<EntityHandle> for InteractMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(INTERACT, |_lua, this, args: Variadic<Value>| {
            push_command(Box::new(CallEntityFnCmd {
                entity: this.entity,
                fn_name: INTERACT.to_string(),
                args: args.to_vec(),
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@vararg any Arguments passed to the entity's interact function");
        out.line("---@return nil");
        out.line(&format!("function Entity:{}(...) end", INTERACT));
        out.line("");
    }
}

pub struct FindBestInteractableMethod;
impl LuaMethod<EntityHandle> for FindBestInteractableMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(FIND_BEST_INTERACTABLE, |lua, _this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            if let Some(entity) = find_best_interactable(ecs) {
                lua_entity_handle(lua, entity)
            } else {
                Ok(Value::Nil)
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@return Entity|nil");
        out.line(&format!("function Entity:{}() end", FIND_BEST_INTERACTABLE));
        out.line("");
    }
}

/// Method: `entity:set_clip("Walk")`
pub struct SetClipMethod;
impl LuaMethod<EntityHandle> for SetClipMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(SET_CLIP, |_lua, this, clip_name: String| {
            push_command(Box::new(SetClipCmd {
                entity: this.entity,
                clip_name,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Sets the active animation clip.");
        out.line("---@param clip_name string The name of the clip (e.g. \"Walk\", \"Idle\")");
        out.line(&format!("function Entity:{}(clip_name) end", SET_CLIP));
        out.line("");
    }
}

/// Method: `entity:get_clip() -> string?`
pub struct GetClipMethod;
impl LuaMethod<EntityHandle> for GetClipMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(GET_CLIP, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;

            if let Some(animation) = ecs.get::<Animation>(this.entity) {
                if let Some(clip_id) = &animation.current {
                    Ok(Value::String(lua.create_string(clip_id.ui_label())?))
                } else {
                    Ok(Value::Nil)
                }
            } else {
                Ok(Value::Nil)
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Gets the current animation clip name.");
        out.line("---@return string|nil");
        out.line(&format!("function Entity:{}() end", GET_CLIP));
        out.line("");
    }
}

/// Method: `entity:reset_clip()`
pub struct ResetClipMethod;
impl LuaMethod<EntityHandle> for ResetClipMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(RESET_CLIP, |_lua, this, ()| {
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

/// Method: `entity:set_flip_x(true)`
pub struct SetFlipXMethod;
impl LuaMethod<EntityHandle> for SetFlipXMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(SET_FLIP_X, |_lua, this, flip_x: bool| {
            push_command(Box::new(SetFlipXCmd {
                entity: this.entity,
                flip_x,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Sets horizontal flip for the sprite.");
        out.line("---@param flip_x boolean Whether to flip horizontally");
        out.line(&format!("function Entity:{}(flip_x) end", SET_FLIP_X));
        out.line("");
    }
}

/// Method: `entity:get_flip_x() -> bool`
pub struct GetFlipXMethod;
impl LuaMethod<EntityHandle> for GetFlipXMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(GET_FLIP_X, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;

            if let Some(animation) = ecs.get::<Animation>(this.entity) {
                Ok(animation.flip_x)
            } else {
                Ok(false)
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Gets the horizontal flip state.");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}() end", GET_FLIP_X));
        out.line("");
    }
}

/// Method: `entity:set_facing("left")`
pub struct SetFacingMethod;
impl LuaMethod<EntityHandle> for SetFacingMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(SET_FACING, |_lua, this, direction: String| {
            push_command(Box::new(SetFacingCmd {
                entity: this.entity,
                direction,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Sets the facing direction (for auto-flip with mirrored clips).");
        out.line("---@param direction string \"left\" or \"right\"");
        out.line(&format!("function Entity:{}(direction) end", SET_FACING));
        out.line("");
    }
}

/// Method: `entity:set_anim_speed(1.5)`
pub struct SetAnimSpeedMethod;
impl LuaMethod<EntityHandle> for SetAnimSpeedMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(SET_ANIM_SPEED, |_lua, this, speed: f32| {
            push_command(Box::new(SetAnimSpeedCmd {
                entity: this.entity,
                speed,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Sets the animation playback speed multiplier.");
        out.line("---@param speed number Speed multiplier (1.0 = normal)");
        out.line(&format!("function Entity:{}(speed) end", SET_ANIM_SPEED));
        out.line("");
    }
}

/// Method: `entity:get_current_frame() -> {col, row}`
pub struct GetCurrentFrameMethod;
impl LuaMethod<EntityHandle> for GetCurrentFrameMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(GET_CURRENT_FRAME, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;

            if let Some(frame) = ecs.get::<CurrentFrame>(this.entity) {
                let table = lua.create_table()?;
                table.set("col", frame.col)?;
                table.set("row", frame.row)?;
                Ok(Value::Table(table))
            } else {
                Ok(Value::Nil)
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Gets the current animation frame indices.");
        out.line("---@return {col: integer, row: integer}|nil");
        out.line(&format!("function Entity:{}() end", GET_CURRENT_FRAME));
        out.line("");
    }
}

/// Method: `entity:is_clip_finished() -> bool`
pub struct IsClipFinishedMethod;
impl LuaMethod<EntityHandle> for IsClipFinishedMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(IS_CLIP_FINISHED, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;

            if let Some(animation) = ecs.get::<Animation>(this.entity) {
                if let Some(current_id) = &animation.current {
                    if let Some(state) = animation.states.get(current_id) {
                        return Ok(state.finished);
                    }
                }
            }
            Ok(false)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Checks if the current non-looping clip has finished.");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}() end", IS_CLIP_FINISHED));
        out.line("");
    }
}

/// Method: `entity:say(dialogue_id, key, opts)`
pub struct SayMethod;
impl LuaMethod<EntityHandle> for SayMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(
            SAY,
            |lua, this, (dialogue_id, key, opts): (String, String, Option<Table>)| {
                let ctx = LuaGameCtx::borrow_ctx(lua)?;
                let game_instance = ctx.game_instance.borrow();
                let config = game_instance.game.text_manager.config.clone();

                let text = match game_instance
                    .game
                    .text_manager
                    .select_text(&dialogue_id, &key)
                {
                    Some(t) => t,
                    None => {
                        log::warn!("Dialogue not found: {}:{}", dialogue_id, key);
                        return Ok(());
                    }
                };
                drop(game_instance);

                let text = if let Some(ref opts_table) = opts {
                    if let Ok(vars_table) = opts_table.get::<Table>("vars") {
                        let mut vars = HashMap::new();
                        for (k, v) in vars_table.pairs::<String, String>().flatten() {
                            vars.insert(k, v);
                        }
                        interpolate(&text, &vars)
                    } else {
                        text
                    }
                } else {
                    text
                };

                let duration = opts
                    .as_ref()
                    .and_then(|t| t.get::<f32>("duration").ok())
                    .unwrap_or(config.default_duration);

                let color = opts.as_ref().and_then(|t| {
                    t.get::<Table>("color").ok().and_then(|c| {
                        Some([
                            c.get::<f32>(1).ok()?,
                            c.get::<f32>(2).ok()?,
                            c.get::<f32>(3).ok()?,
                            c.get::<f32>(4).ok().unwrap_or(1.0),
                        ])
                    })
                });

                let offset = opts.as_ref().and_then(|t| {
                    t.get::<Table>("offset")
                        .ok()
                        .and_then(|o| Some((o.get::<f32>(1).ok()?, o.get::<f32>(2).ok()?)))
                });

                let font_size = opts.as_ref().and_then(|t| t.get::<f32>("font_size").ok());
                let max_width = opts.as_ref().and_then(|t| t.get::<f32>("max_width").ok());
                let show_background = opts
                    .as_ref()
                    .and_then(|t| t.get::<bool>("show_background").ok());

                let background_color = opts.as_ref().and_then(|t| {
                    t.get::<Table>("background_color").ok().and_then(|c| {
                        Some([
                            c.get::<f32>(1).ok()?,
                            c.get::<f32>(2).ok()?,
                            c.get::<f32>(3).ok()?,
                            c.get::<f32>(4).ok().unwrap_or(0.7),
                        ])
                    })
                });

                push_command(Box::new(ShowSpeechCmd {
                    entity: this.entity,
                    text,
                    duration,
                    color,
                    offset,
                    font_size,
                    max_width,
                    show_background,
                    background_color,
                }));
                Ok(())
            },
        );
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Shows a speech bubble with text from a dialogue file.");
        out.line("---@param dialogue_id string The dialogue file ID (e.g. \"npc_merchant\")");
        out.line("---@param key string The dialogue key (e.g. \"greeting\")");
        out.line("---@param opts? {vars?: table<string, string>, duration?: number, color?: number[], offset?: number[], font_size?: number, max_width?: number, show_background?: boolean, background_color?: number[]}");
        out.line(&format!(
            "function Entity:{}(dialogue_id, key, opts) end",
            SAY
        ));
        out.line("");
    }
}

/// Method: `entity:clear_speech()`
pub struct ClearSpeechMethod;
impl LuaMethod<EntityHandle> for ClearSpeechMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(CLEAR_SPEECH, |_lua, this, ()| {
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

/// Method: `entity:is_speaking() -> bool`
pub struct IsSpeakingMethod;
impl LuaMethod<EntityHandle> for IsSpeakingMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(IS_SPEAKING, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            Ok(ecs.has::<SpeechBubble>(this.entity))
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Checks if the entity currently has a speech bubble.");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}() end", IS_SPEAKING));
        out.line("");
    }
}

/// Method: `entity:play_sound(group_name)`
pub struct PlaySoundMethod;
impl LuaMethod<EntityHandle> for PlaySoundMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(ENTITY_PLAY_SOUND, |lua, this, group_name: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let Some(source) = ecs.get::<AudioSource>(this.entity) else {
                return Ok(());
            };

            let group_id = SoundGroupId::Custom(group_name.clone());
            let Some(group) = source.groups.get(&group_id) else {
                log::warn!(
                    "Entity {:?} tried to play missing sound group '{}'",
                    this.entity,
                    group_name
                );
                return Ok(());
            };
            let volume = (group.volume * source.runtime_volume).clamp(0.0, 1.0);

            if group.looping {
                push_audio_command(AudioCommand::PlayLoop {
                    handle: *this.entity as u64,
                    sounds: group.sounds.clone(),
                    volume,
                    pitch_variation: group.pitch_variation,
                    volume_variation: group.volume_variation,
                });
            } else {
                push_audio_command(AudioCommand::PlayVariedSfx {
                    sounds: group.sounds.clone(),
                    volume,
                    pitch_variation: group.pitch_variation,
                    volume_variation: group.volume_variation,
                });
            }
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(
            "--- Plays the named sound group configured on this entity's AudioSource component.",
        );
        out.line("--- If the group is looping, starts a loop tracked by the entity ID.");
        out.line("--- If one-shot, plays with the group's pitch and volume variation.");
        out.line("---@param group_name SoundGroupId");
        out.line(&format!(
            "function Entity:{}(group_name) end",
            ENTITY_PLAY_SOUND
        ));
        out.line("");
    }
}

/// Method: `entity:stop_sound()`
pub struct StopSoundMethod;
impl LuaMethod<EntityHandle> for StopSoundMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(ENTITY_STOP_SOUND, |_lua, this, ()| {
            push_audio_command(AudioCommand::StopLoop(*this.entity as u64));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Stops a looping sound started by this entity's AudioSource.");
        out.line(&format!("function Entity:{}() end", ENTITY_STOP_SOUND));
        out.line("");
    }
}

/// Method: `entity:set_sound_volume(v)`
pub struct SetSoundVolumeMethod;
impl LuaMethod<EntityHandle> for SetSoundVolumeMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(ENTITY_SET_SOUND_VOLUME, |lua, this, v: f32| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let mut game_instance = ctx.game_instance.borrow_mut();
            let ecs = &mut game_instance.game.ecs;
            if let Some(source) = ecs.get_mut::<AudioSource>(this.entity) {
                source.runtime_volume = v.clamp(0.0, 1.0);
            }
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(
            "--- Sets a runtime gain multiplier on this entity's AudioSource groups (0.0–1.0).",
        );
        out.line("--- Takes effect on the next play_sound() call.");
        out.line("---@param v number Volume in range 0.0–1.0");
        out.line(&format!(
            "function Entity:{}(v) end",
            ENTITY_SET_SOUND_VOLUME
        ));
        out.line("");
    }
}
