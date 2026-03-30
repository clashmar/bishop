use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum StartupMode {
    Full,
    #[default]
    Skip,
}

impl StartupMode {
    /// Returns the other available startup mode.
    pub fn toggled(self) -> Self {
        match self {
            Self::Full => Self::Skip,
            Self::Skip => Self::Full,
        }
    }
}

impl fmt::Display for StartupMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => f.write_str("Full"),
            Self::Skip => f.write_str("Skip"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggled_switches_between_full_and_skip() {
        assert_eq!(StartupMode::Full.toggled(), StartupMode::Skip);
        assert_eq!(StartupMode::Skip.toggled(), StartupMode::Full);
    }
}
