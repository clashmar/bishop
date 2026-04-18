use super::*;
use crate::audio::command_queue::{drain_audio_commands, push_audio_command, PlayMusicRequest};
use crate::ecs::{AudioGroup, SoundGroupId};

mod audio_source_tests;

#[test]
fn play_music_request_can_be_queued_and_drained() {
    let _ = drain_audio_commands();

    push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
        id: "music/intro".to_string(),
        looping: false,
        fade_out: 0.5,
        gap: 0.25,
        fade_in: 0.75,
    }));

    let commands = drain_audio_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        AudioCommand::PlayMusic(request) => {
            assert_eq!(request.id, "music/intro");
            assert!(!request.looping);
            assert_eq!(request.fade_out, 0.5);
            assert_eq!(request.gap, 0.25);
            assert_eq!(request.fade_in, 0.75);
        }
        _ => panic!("expected PlayMusic"),
    }
}

#[cfg(feature = "editor")]
#[test]
fn tracked_preview_commands_can_be_queued_and_drained() {
    let _ = drain_audio_commands();

    push_audio_command(AudioCommand::PlayTrackedPreview {
        handle: 7,
        sounds: vec!["ui/click".to_string()],
        volume: 0.75,
        pitch_variation: 0.1,
        volume_variation: 0.2,
        looping: true,
        timeout: 1.5,
    });
    push_audio_command(AudioCommand::StopTrackedPreview(7));

    let commands = drain_audio_commands();
    assert_eq!(commands.len(), 2);
    match &commands[0] {
        AudioCommand::PlayTrackedPreview {
            handle,
            sounds,
            volume,
            pitch_variation,
            volume_variation,
            looping,
            timeout,
        } => {
            assert_eq!(*handle, 7);
            assert_eq!(sounds, &vec!["ui/click".to_string()]);
            assert_eq!(*volume, 0.75);
            assert_eq!(*pitch_variation, 0.1);
            assert_eq!(*volume_variation, 0.2);
            assert!(*looping);
            assert_eq!(*timeout, 1.5);
        }
        _ => panic!("expected PlayTrackedPreview"),
    }
    match &commands[1] {
        AudioCommand::StopTrackedPreview(handle) => assert_eq!(*handle, 7),
        _ => panic!("expected StopTrackedPreview"),
    }
}

#[test]
fn sound_group_id_ui_label_uses_custom_name() {
    assert_eq!(
        SoundGroupId::Custom("Footsteps".to_string()).ui_label(),
        "Footsteps"
    );
    assert_eq!(SoundGroupId::New.ui_label(), "Add Group");
}

#[test]
fn audio_group_defaults_to_full_volume() {
    assert_eq!(AudioGroup::default().volume, 1.0);
}
