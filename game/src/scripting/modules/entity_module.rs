use engine_core::prelude::*;
use engine_core::scripting::lua_constants::{lua_fields, lua_files, lua_globals};
use mlua::prelude::LuaResult;
use mlua::{Lua, UserData, UserDataMethods, UserDataRegistry};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

mod commands;
mod handle;
mod queries;

use commands::*;
pub use handle::{lua_entity_handle, EntityHandle};
use queries::*;

macro_rules! entity_handle_methods {
    ($($variant:ident => $method:ident),+ $(,)?) => {
        #[derive(Clone, Copy, EnumIter)]
        enum EntityHandleMethod {
            $($variant),+
        }

        impl LuaMethod<EntityHandle> for EntityHandleMethod {
            fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
                match self {
                    $(EntityHandleMethod::$variant => $method.register(methods),)+
                }
            }

            fn emit_api(&self, out: &mut LuaApiWriter) {
                match self {
                    $(EntityHandleMethod::$variant => $method.emit_api(out),)+
                }
            }
        }
    };
}

entity_handle_methods! {
    Despawn => DespawnMethod,
    Get => GetMethod,
    Set => SetMethod,
    Has => HasMethod,
    Interact => InteractMethod,
    FindBestInteractable => FindBestInteractableMethod,
    SetClip => SetClipMethod,
    GetClip => GetClipMethod,
    ResetClip => ResetClipMethod,
    SetFlipX => SetFlipXMethod,
    GetFlipX => GetFlipXMethod,
    SetFacing => SetFacingMethod,
    SetAnimSpeed => SetAnimSpeedMethod,
    Teleport => TeleportMethod,
    MoveBy => MoveByMethod,
    GetCurrentFrame => GetCurrentFrameMethod,
    IsClipFinished => IsClipFinishedMethod,
    Say => SayMethod,
    ClearSpeech => ClearSpeechMethod,
    IsSpeaking => IsSpeakingMethod,
    PlaySound => PlaySoundMethod,
    StopSound => StopSoundMethod,
    SetSoundVolume => SetSoundVolumeMethod,
}

#[derive(Default)]
pub struct EntityModule;
register_lua_module!(EntityModule);
register_lua_api!(EntityModule, lua_files::ENTITY);

impl LuaModule for EntityModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let factory =
            lua.create_function(|_, id: usize| Ok(EntityHandle { entity: Entity(id) }))?;
        lua.globals().set(lua_globals::ENTITY, factory)?;
        Ok(())
    }
}

impl LuaApi for EntityModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@class Entity");
        out.line("---@field id integer");
        out.line("local Entity = {}");
        out.line("");

        for method in EntityHandleMethod::iter() {
            method.emit_api(out);
        }

        out.line("return Entity");
    }
}

impl UserData for EntityHandle {
    fn add_methods<'lua, M: UserDataMethods<Self>>(methods: &mut M) {
        for method in EntityHandleMethod::iter() {
            method.register(methods);
        }
    }

    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get(lua_fields::ID, |_, this| Ok(*this.entity));
    }

    fn register(registry: &mut UserDataRegistry<Self>) {
        Self::add_fields(registry);
        Self::add_methods(registry);
    }
}
