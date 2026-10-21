use std::fs;
use std::sync::Arc;

use mlua::{Function, Lua, Value};

use crate::storage::path_utils::{scripts_folder, themes_folder};

const INIT_FILE: &str = "init.lua";
const FILE_REQUIRE_INSTALLED: &str = "bishop:file_require_installed";

/// Configures game script and theme modules for Lua require.
pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    if lua.named_registry_value::<Option<bool>>(FILE_REQUIRE_INSTALLED)?.unwrap_or(false) {
        return Ok(());
    }
    let original_require: Function = lua.globals().get("require")?;
    let cache = lua.create_table()?;
    let require = lua.create_function(move |lua, module: String| {
        if module.starts_with('@') || module.starts_with("./") || module.starts_with("../") {
            return original_require.call(module);
        }
        if let Some(value) = cache.get::<Option<Value>>(module.as_str())? {
            return Ok(value);
        }
        let relative = module.replace('.', "/");
        let module_path = scripts_folder().join(&relative);
        let source = module_path.with_extension("lua");
        let init = module_path.join(INIT_FILE);
        let theme = themes_folder().join(relative).with_extension("lua");
        let path = if source.is_file() {
            source
        } else if init.is_file() {
            init
        } else if theme.is_file() {
            theme
        } else {
            return Err(mlua::Error::RuntimeError(format!(
                "module '{module}' not found; searched {}, {}, {}",
                source.display(), init.display(), theme.display()
            )));
        };
        let src = fs::read_to_string(&path)
            .map_err(|error| mlua::Error::ExternalError(Arc::new(error)))?;
        cache.set(module.as_str(), true)?;
        let value = match lua.load(&src).set_name(path.to_string_lossy().into_owned()).eval::<Value>() {
            Ok(Value::Nil) => Value::Boolean(true),
            Ok(value) => value,
            Err(error) => {
                cache.set(module.as_str(), Value::Nil)?;
                return Err(error);
            }
        };
        cache.set(module, value.clone())?;
        Ok(value)
    })?;
    lua.globals().set("require", require)?;
    lua.set_named_registry_value(FILE_REQUIRE_INSTALLED, true)
}

#[cfg(test)]
mod tests;
