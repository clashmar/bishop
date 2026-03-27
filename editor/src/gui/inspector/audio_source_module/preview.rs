use super::*;
use engine_core::audio::command_queue::{push_audio_command, AudioCommand};
use std::cell::RefCell;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PreviewRequest {
    row_index: usize,
    sound_id: String,
}

impl PreviewRequest {
    pub(super) fn new(row_index: usize, sound_id: String) -> Self {
        Self { row_index, sound_id }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ActivePreview {
    pub(super) entity: Entity,
    pub(super) group_id: SoundGroupId,
    pub(super) request: PreviewRequest,
    pub(super) remaining_seconds: f32,
}

thread_local! {
    static ACTIVE_AUDIO_PREVIEW: RefCell<Option<ActivePreview>> = const { RefCell::new(None) };
}

pub fn clear_active_audio_preview() {
    ACTIVE_AUDIO_PREVIEW.with(|active| {
        if active.borrow_mut().take().is_some() {
            push_audio_command(AudioCommand::StopTrackedPreview(PREVIEW_HANDLE));
        }
    });
}

pub(super) fn tick_active_audio_preview(dt: f32) {
    let expired = ACTIVE_AUDIO_PREVIEW.with(|active| {
        let mut active = active.borrow_mut();
        let Some(preview) = active.as_mut() else {
            return false;
        };

        preview.remaining_seconds -= dt.max(0.0);
        preview.remaining_seconds <= 0.0
    });

    if expired {
        clear_active_audio_preview();
    }
}

pub(super) fn sync_active_preview(
    entity: Entity,
    group_id: &SoundGroupId,
    sounds: &[String],
) {
    let should_clear = ACTIVE_AUDIO_PREVIEW.with(|active| {
        active.borrow().as_ref().is_some_and(|preview| {
            preview.entity == entity
                && (preview.group_id != *group_id
                    || !sounds.iter().enumerate().any(|(index, sound)| {
                        preview.request.row_index == index && preview.request.sound_id == *sound
                    }))
        })
    });

    if should_clear {
        clear_active_audio_preview();
    }
}

pub(super) fn apply_preview_request(
    entity: Entity,
    group_id: &SoundGroupId,
    next_preview: Option<PreviewRequest>,
    group: &AudioGroup,
) {
    match next_preview {
        Some(request) => {
            push_audio_command(AudioCommand::PlayTrackedPreview {
                handle: PREVIEW_HANDLE,
                sounds: vec![request.sound_id.clone()],
                volume: group.volume,
                pitch_variation: group.pitch_variation,
                volume_variation: group.volume_variation,
                looping: false,
                timeout: PREVIEW_TIMEOUT_SECONDS,
            });
            ACTIVE_AUDIO_PREVIEW.with(|active| {
                *active.borrow_mut() = Some(ActivePreview {
                    entity,
                    group_id: group_id.clone(),
                    request,
                    remaining_seconds: PREVIEW_TIMEOUT_SECONDS,
                });
            });
        }
        None => clear_active_audio_preview(),
    }
}

#[cfg(test)]
pub(super) fn set_active_preview_for_test(preview: Option<ActivePreview>) {
    ACTIVE_AUDIO_PREVIEW.with(|active| {
        *active.borrow_mut() = preview;
    });
}

#[cfg(test)]
pub(super) fn active_preview_is_cleared_for_test() -> bool {
    ACTIVE_AUDIO_PREVIEW.with(|active| active.borrow().is_none())
}
