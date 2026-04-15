pub mod audio_source;
pub mod command_queue;
pub mod diagnostics;
pub mod loader;
mod manager;
pub mod runtime;
#[cfg(test)]
mod tests;

pub use audio_source::{AudioGroup, AudioSource, SoundGroupId, SoundPresetLink};
pub use command_queue::{clear_audio_commands, push_audio_command, AudioCommand, PlayMusicRequest};
pub use diagnostics::{AudioDiagnosticsEntry, AudioDiagnosticsSnapshot};
pub use loader::load_wav;
pub use manager::AudioManager;
pub use runtime::{reset_audio_runtime_state, MusicStopReason, MusicStoppedEvent};
