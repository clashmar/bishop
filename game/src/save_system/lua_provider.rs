use std::io;

use mlua::{Function, Lua, RegistryKey};

use crate::save_system::{RestorePhase, SaveProvider, SaveProviderId, SavedSection};
use crate::scripting::error_utils::lua_io_error;

/// A [`SaveProvider`] backed by Lua callbacks.
pub struct LuaSaveProvider {
    id: SaveProviderId,
    version: u32,
    lua: Lua,
    capture_key: RegistryKey,
    apply_key: RegistryKey,
}

impl LuaSaveProvider {
    /// Creates a new `LuaSaveProvider`.
    pub fn new(
        lua: &Lua,
        id: impl Into<String>,
        version: u32,
        capture: Function,
        apply: Function,
    ) -> mlua::Result<Self> {
        Ok(Self {
            id: SaveProviderId::new(id),
            version,
            lua: lua.clone(),
            capture_key: lua.create_registry_value(capture)?,
            apply_key: lua.create_registry_value(apply)?,
        })
    }
}

impl SaveProvider for LuaSaveProvider {
    fn id(&self) -> SaveProviderId {
        self.id.clone()
    }

    fn restore_phase(&self) -> RestorePhase {
        RestorePhase::PostRuntime
    }

    fn capture(&mut self) -> io::Result<SavedSection> {
        let capture: Function = self
            .lua
            .registry_value(&self.capture_key)
            .map_err(lua_io_error)?;
        let data = capture.call::<String>(()).map_err(lua_io_error)?;
        Ok(SavedSection {
            version: self.version,
            data,
        })
    }

    fn apply(&mut self, section: &SavedSection) -> io::Result<()> {
        let apply: Function = self
            .lua
            .registry_value(&self.apply_key)
            .map_err(lua_io_error)?;
        apply.call::<()>(section.data.clone()).map_err(lua_io_error)
    }
}
