use bishop::prelude::*;
use engine_core::constants::playtest_artifacts;
use engine_core::logging::LOG_HISTORY;
use engine_core::onscreen_error;
use engine_core::playtest::{
    build_snapshot_payload, payload_value, PlaytestControlProfile, PlaytestSessionState,
    PlaytestSnapshot, PlaytestSnapshotRequest,
};
use engine_core::prelude::{CameraManager, Transform, Velocity};
use game_lib::engine::{Engine, GameState};
use game_lib::game_global::{
    has_active_playtest_control_timeline, has_pending_completed_playtest_control_frame,
};
use game_lib::playtest::control::AcceptedPlaytestControlRequest;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ActiveControlSnapshotState {
    pub(crate) accepted: AcceptedPlaytestControlRequest,
    next_frame_index: u64,
    total_frames: u64,
}

impl ActiveControlSnapshotState {
    pub(crate) fn from_accepted(accepted: AcceptedPlaytestControlRequest) -> Option<Self> {
        Some(Self {
            total_frames: control_profile_frame_budget(&accepted.profile),
            accepted,
            next_frame_index: 0,
        })
    }

    pub(crate) fn advance_after_snapshot(
        &mut self,
        game_state: &GameState,
        control_still_active: bool,
    ) -> bool {
        if !control_still_active {
            return false;
        }

        if *game_state != GameState::Playing {
            return true;
        }

        self.next_frame_index += 1;
        self.next_frame_index < self.total_frames
    }
}

#[derive(Serialize)]
pub(crate) struct PlaytestRuntimePayload {
    pub(crate) accumulator_ms: f32,
    pub(crate) player_velocity_x: f32,
    pub(crate) player_position_x: f32,
    pub(crate) player_position_y: f32,
    pub(crate) active_control_profile: Option<String>,
    pub(crate) active_control_seed: Option<u64>,
    pub(crate) active_control_frame_index: Option<u64>,
    pub(crate) camera_target: Option<(f32, f32)>,
    pub(crate) camera_zoom: Option<(f32, f32)>,
    pub(crate) camera_follow_enabled: Option<bool>,
    pub(crate) camera_override_active: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LiveCameraState {
    pub(crate) target: (f32, f32),
    pub(crate) zoom: (f32, f32),
    pub(crate) follow_enabled: bool,
    pub(crate) override_active: bool,
}

pub(crate) fn make_snapshot(
    ctx: &PlatformContext,
    engine: &Engine,
    frame_index: u64,
    request: &PlaytestSnapshotRequest,
    active_control_runtime: Option<&ActiveControlSnapshotState>,
) -> PlaytestSnapshot {
    let frame_time_ms = ctx.borrow().get_frame_time() * 1000.0;
    let smoothed_frame_time_ms = engine.smoothed_dt.map(|dt| dt * 1000.0);
    let game_instance = engine.game_instance.borrow();
    let ecs = &game_instance.game.ecs;
    let player_entity = ecs.get_player_entity();
    let player_velocity_x = player_entity
        .and_then(|entity| ecs.get::<Velocity>(entity).copied())
        .map(|velocity| velocity.x)
        .unwrap_or_default();
    let (player_position_x, player_position_y) = player_entity
        .and_then(|entity| ecs.get::<Transform>(entity).copied())
        .map(|transform| (transform.position.x, transform.position.y))
        .unwrap_or_default();
    let recent_log_count = match LOG_HISTORY.lock() {
        Ok(history) => history.total_pushed(),
        Err(_) => 0,
    };
    let (active_control_profile, active_control_seed, active_control_frame_index) =
        active_control_snapshot_fields(
            active_control_runtime,
            has_active_playtest_control_timeline()
                || has_pending_completed_playtest_control_frame(),
        );
    let (camera_target, camera_zoom, camera_follow_enabled, camera_override_active) =
        live_camera_state(&engine.camera_manager)
            .map(|state| {
                (
                    Some(state.target),
                    Some(state.zoom),
                    Some(state.follow_enabled),
                    Some(state.override_active),
                )
            })
            .unwrap_or((None, None, None, None));
    let runtime_payload = match payload_value(PlaytestRuntimePayload {
        accumulator_ms: engine.accumulator * 1000.0,
        player_velocity_x,
        player_position_x,
        player_position_y,
        active_control_profile,
        active_control_seed,
        active_control_frame_index,
        camera_target,
        camera_zoom,
        camera_follow_enabled,
        camera_override_active,
    }) {
        Ok(payload) => Some(payload),
        Err(error) => {
            onscreen_error!("Failed to serialize playtest runtime payload: {error:?}");
            None
        }
    };
    let payload = build_snapshot_payload(request, runtime_payload);

    PlaytestSnapshot {
        session_state: PlaytestSessionState::Running,
        frame_time_ms: Some(frame_time_ms),
        smoothed_frame_time_ms,
        mode: Some(playtest_artifacts::PLAYTEST_MODE.to_string()),
        recent_log_count,
        frame_index: Some(frame_index),
        topic: Some(playtest_artifacts::PLAYTEST_RUNTIME_TOPIC.to_string()),
        label: Some(playtest_artifacts::PLAYTEST_FRAME_LABEL.to_string()),
        payload,
    }
}

pub(crate) fn advance_active_control_runtime_for_next_snapshot(
    active_control_runtime: &mut Option<ActiveControlSnapshotState>,
    game_state: &GameState,
) {
    let control_still_active = has_active_playtest_control_timeline();
    let should_keep = active_control_runtime
        .as_mut()
        .is_some_and(|runtime| runtime.advance_after_snapshot(game_state, control_still_active));

    if !should_keep {
        *active_control_runtime = None;
    }
}

pub(crate) fn active_control_snapshot_fields(
    active_control_runtime: Option<&ActiveControlSnapshotState>,
    control_still_active: bool,
) -> (Option<String>, Option<u64>, Option<u64>) {
    if !control_still_active {
        return (None, None, None);
    }

    let Some(runtime) = active_control_runtime else {
        return (None, None, None);
    };

    (
        Some(runtime.accepted.profile_label.clone()),
        runtime
            .accepted
            .request
            .chaos
            .as_ref()
            .map(|chaos| chaos.seed),
        Some(runtime.next_frame_index),
    )
}

pub(crate) fn live_camera_state(camera_manager: &CameraManager) -> Option<LiveCameraState> {
    let target = camera_manager.active.camera.target;
    let zoom = camera_manager.active.camera.zoom;

    if !target.x.is_finite() || !target.y.is_finite() || !zoom.x.is_finite() || !zoom.y.is_finite()
    {
        return None;
    }

    Some(LiveCameraState {
        target: (target.x, target.y),
        zoom: (zoom.x, zoom.y),
        follow_enabled: camera_manager.follow_is_enabled(),
        override_active: camera_manager.runtime_override_is_active(),
    })
}

fn control_profile_frame_budget(profile: &PlaytestControlProfile) -> u64 {
    let movement_frames = profile
        .movement_frames
        .iter()
        .map(|frame| u64::from(frame.frame_count))
        .sum::<u64>();
    let camera_frames = profile
        .camera_frames
        .iter()
        .map(|frame| u64::from(frame.frame_count))
        .sum::<u64>();

    movement_frames.max(camera_frames)
}
