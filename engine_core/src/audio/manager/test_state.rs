use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct StartedOneShotPlayback {
    pub(super) id: String,
    pub(super) volume: f32,
    pub(super) pitch: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct StartedLoopPlayback {
    pub(super) id: String,
    pub(super) volume: f32,
    pub(super) pitch: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct StartedTrackedPreviewPlayback {
    pub(super) id: String,
    pub(super) volume: f32,
    pub(super) pitch: f32,
    pub(super) looping: bool,
}

#[derive(Default)]
pub(super) struct AudioManagerTestState {
    pub(super) started_one_shot_playbacks: Vec<StartedOneShotPlayback>,
    pub(super) active_loop_sound_ids: HashMap<u64, String>,
    pub(super) started_loop_playbacks: HashMap<u64, StartedLoopPlayback>,
    pub(super) started_tracked_preview_playbacks: HashMap<u64, StartedTrackedPreviewPlayback>,
}
