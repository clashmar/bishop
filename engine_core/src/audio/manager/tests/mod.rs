mod diagnostics_tests;
mod music_load_tests;
mod music_transition_tests;
mod prefetched_tests;
#[cfg(feature = "editor")]
mod preview_tests;
mod sfx_tests;

use super::*;
use crate::audio::command_queue::{drain_audio_commands, push_audio_command};
use crate::audio::runtime;
use crate::audio::{
    AudioCommand, AudioDiagnosticsEntry, AudioLoopKey, AudioPlaybackOwner, PlayMusicRequest,
};
use crate::audio::test_utils::TestBackend;
use crate::ecs::{Entity, SoundGroupId};
use crate::hydration::Hydratable;
use crate::task::BackgroundService;
use crate::worlds::{RoomId, WorldId};
use oddio::Frames;

fn seeded_manager() -> AudioManager {
    runtime::reset_for_tests();
    let _ = drain_audio_commands();

    let mut manager = AudioManager::new::<TestBackend>();
    manager.sound_cache.insert(
        "preview/click".to_string(),
        Frames::from_slice(44_100, &[[0.0, 0.0]]),
    );
    manager.sound_cache.insert(
        "music/intro".to_string(),
        Frames::from_slice(10, &[[0.0, 0.0]; 10]),
    );
    manager.sound_cache.insert(
        "music/next".to_string(),
        Frames::from_slice(10, &[[0.0, 0.0]; 20]),
    );
    manager
}

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

fn entity_loop_key(entity_id: usize, group_name: &str) -> AudioLoopKey {
    AudioLoopKey::new(
        AudioPlaybackOwner::Entity(Entity(entity_id)),
        SoundGroupId::Custom(group_name.to_string()),
    )
}

