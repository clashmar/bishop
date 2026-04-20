use mlua::{FromLua, Lua, UserData, Value};
use serde::{Deserialize, Serialize};

/// Opaque handle that identifies a managed text TOML asset. Default/Unset is 0.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize, Default,
)]
pub struct TomlId(pub usize);

impl UserData for TomlId {}

impl FromLua for TomlId {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(user_data) => Ok(*user_data.borrow::<TomlId>()?),
            other => Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "TomlId".to_string(),
                message: Some("expected TomlId userdata".to_string()),
            }),
        }
    }
}
