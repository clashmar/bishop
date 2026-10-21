use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;

use mlua::{Function, Lua, Value};

use crate::storage::path_utils::{scripts_folder, themes_folder};

const INIT_FILE: &str = "init.lua";
const FILE_REQUIRE_INSTALLED: &str = "bishop:file_require_installed";

/// Configures game script and theme modules for Lua require.
pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    install_source(lua, ModuleSource::Filesystem)
}

/// Installs the normal Luau resolver using already-loaded game modules.
pub fn install_prefetched(lua: &Lua, files: HashMap<String, String>) -> mlua::Result<()> {
    install_source(lua, ModuleSource::Prefetched(files))
}

fn install_source(lua: &Lua, source: ModuleSource) -> mlua::Result<()> {
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
        let candidates = module_candidates(&module);
        let mut selected = None;
        for candidate in &candidates {
            if let Some(src) = source.read(candidate)? {
                selected = Some((source.path(candidate), src));
                break;
            }
        }
        let (path, src) = selected.ok_or_else(|| {
            mlua::Error::RuntimeError(format!(
                "module '{module}' not found; searched {}, {}, {}",
                source.path(&candidates[0]).display(),
                source.path(&candidates[1]).display(),
                source.path(&candidates[2]).display()
            ))
        })?;
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

enum ModuleSource {
    Filesystem,
    Prefetched(HashMap<String, String>),
}

impl ModuleSource {
    fn path(&self, candidate: &str) -> PathBuf {
        match self {
            Self::Filesystem => match candidate.strip_prefix("scripts/") {
                Some(relative) => scripts_folder().join(relative),
                None => themes_folder().join(candidate.strip_prefix("themes/").unwrap()),
            },
            Self::Prefetched(_) => PathBuf::from(candidate),
        }
    }

    fn read(&self, candidate: &str) -> mlua::Result<Option<String>> {
        match self {
            Self::Filesystem => match fs::read_to_string(self.path(candidate)) {
                Ok(src) => Ok(Some(src)),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
                Err(error) => Err(mlua::Error::ExternalError(Arc::new(error))),
            },
            Self::Prefetched(files) => Ok(files.get(candidate).cloned()),
        }
    }
}

fn module_candidates(module: &str) -> [String; 3] {
    let path = module.replace('.', "/");
    [
        format!("scripts/{path}.lua"),
        format!("scripts/{path}/{INIT_FILE}"),
        format!("themes/{path}.lua"),
    ]
}

#[cfg(test)]
mod tests;
