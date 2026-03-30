use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum StartupMode {
    Full,
    #[default]
    Skip,
}

impl fmt::Display for StartupMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => f.write_str("Full"),
            Self::Skip => f.write_str("Skip"),
        }
    }
}
