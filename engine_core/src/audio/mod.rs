pub mod command_queue;
pub mod diagnostics;
pub mod loader;
mod manager;
pub mod runtime;
#[cfg(test)]
pub(crate) mod test_utils;
#[cfg(test)]
mod tests;

pub use command_queue::{push_audio_command, AudioCommand, PlayMusicRequest};
pub use diagnostics::{AudioDiagnosticsEntry, AudioDiagnosticsSnapshot};
pub use loader::load_wav;
pub use manager::AudioManager;
pub use runtime::{MusicStopReason, MusicStoppedEvent};
