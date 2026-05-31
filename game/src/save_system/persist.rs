use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::io::{self, Error};
use std::path::Path;

/// Shared RON read/write behaviour for save-related types.
pub trait RonPersist: Serialize + DeserializeOwned + Sized {
    /// RON pretty-print config used for serialization.
    fn ron_config() -> ron::ser::PrettyConfig {
        ron::ser::PrettyConfig::new()
    }

    fn to_ron_string(&self) -> io::Result<String> {
        ron::ser::to_string_pretty(self, Self::ron_config()).map_err(Error::other)
    }

    fn from_ron_str(ron: &str) -> io::Result<Self> {
        ron::from_str(ron).map_err(Error::other)
    }

    fn write_to_path(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_ron_string()?)
    }

    fn read_from_path(path: &Path) -> io::Result<Self> {
        let ron = fs::read_to_string(path)?;
        Self::from_ron_str(&ron)
    }
}
