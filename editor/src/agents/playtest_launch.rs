use crate::playtest::room_playtest::resolve_playtest_binary;
use engine_core::constants::agents;
use std::ffi::OsString;
use std::path::PathBuf;

/// Canonical launch data for a seeded agent playtest run.
pub struct SeededAgentPlaytestLaunch {
    /// Resolved `game-playtest` executable path.
    pub exe_path: PathBuf,
    /// Seeded payload file to launch.
    pub payload_path: PathBuf,
    /// Canonical headless playtest arguments.
    pub args: [OsString; 3],
}

impl SeededAgentPlaytestLaunch {
    /// Returns borrowed argument references for process spawning.
    pub fn arg_refs(&self) -> [&std::ffi::OsStr; 3] {
        [
            self.args[0].as_os_str(),
            self.args[1].as_os_str(),
            self.args[2].as_os_str(),
        ]
    }
}

/// Builds canonical launch data for an already-written seeded payload path.
pub fn build_seeded_agent_playtest_launch(
    payload_path: PathBuf,
) -> Result<SeededAgentPlaytestLaunch, String> {
    let exe_path = resolve_playtest_binary().map_err(|error| error.to_string())?;
    let args = [
        OsString::from(agents::HEADLESS_FLAG),
        OsString::from(agents::PAYLOAD_FLAG),
        payload_path.as_os_str().to_os_string(),
    ];

    Ok(SeededAgentPlaytestLaunch {
        exe_path,
        payload_path,
        args,
    })
}
