use serde::{Deserialize, Serialize};

/// Opaque handle that identifies a managed text TOML asset. Default/Unset is 0.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize, Default,
)]
pub struct TomlId(pub usize);
