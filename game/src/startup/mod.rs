mod asset;
mod bootstrap;
mod playtest_args;
mod rebootstrap;
mod runtime_icon;
#[cfg(test)]
mod tests;

pub use asset::{
    load_startup_for_game_name, load_startup_from_resources, LoadingConfig,
    StartupAsset, StartupScreenContent, StartupScreenSpec,
};
pub use bootstrap::{StartupController, StartupSource};
pub use playtest_args::PlaytestLaunchArgs;
pub use rebootstrap::{entry_mode_for_intent, StartupIntent, StartupRequest};
pub use runtime_icon::{
    playtest_game_name_from_payload, runtime_icon_for_current_exe,
    runtime_icon_for_playtest_payload,
};
