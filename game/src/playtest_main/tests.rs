use super::agent_session::AgentPlaytestSession;
use super::snapshot::{
    active_control_snapshot_fields, advance_active_control_runtime_for_next_snapshot,
    live_camera_state, ActiveControlSnapshotState, LiveCameraState, PlaytestRuntimePayload,
};
use super::PlaytestApp;
use crate::playtest::FilePlaytestSessionTransport;
use bishop::vec2;
use engine_core::input::input_constants;
use engine_core::payload;
use engine_core::playtest::{
    ControlLoopPolicy, ControlStartPolicy, MovementControlFrame, PlaytestActiveControl,
    PlaytestChaosConfig, PlaytestControlProfile, PlaytestControlProfileRef, PlaytestControlRequest,
    PlaytestSessionManifest, PlaytestSnapshotRequest, BUILTIN_PROFILE_CAMERA_PAN_SWEEP,
    BUILTIN_PROFILE_GROUNDED_WALK_RIGHT,
};
use engine_core::prelude::CameraManager;
use game_lib::engine::GameState;
use game_lib::game_global::{
    clear_active_playtest_control_timeline, clear_virtual_input_state, get_virtual_input_state,
    install_active_playtest_control_timeline, tick_active_playtest_control_timeline,
};
use game_lib::playtest::control::{accept_playtest_control_request, ActiveControlTimeline};
use game_lib::startup::PlaytestLaunchArgs;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

struct RuntimeControlStateGuard;

impl RuntimeControlStateGuard {
    fn new() -> Self {
        clear_virtual_input_state();
        clear_active_playtest_control_timeline();
        Self
    }
}

impl Drop for RuntimeControlStateGuard {
    fn drop(&mut self) {
        clear_virtual_input_state();
        clear_active_playtest_control_timeline();
    }
}

#[test]
fn runtime_control_request_updates_manifest_and_snapshot_state() {
    let transport =
        FilePlaytestSessionTransport::new(std::env::temp_dir().join("playtest-main-plan"));
    let accepted = accept_playtest_control_request(PlaytestControlRequest::named(
        BUILTIN_PROFILE_GROUNDED_WALK_RIGHT,
    ))
    .unwrap();
    let mut session = AgentPlaytestSession::unattached(
        "session-1".to_string(),
        PlaytestSnapshotRequest::default(),
        None,
    );
    session.attach_transport(transport.clone());
    session.initialize_manifest("payload.ron".to_string());

    session.apply_runtime_control(accepted.clone());

    let manifest_ron = fs::read_to_string(transport.manifest_path()).unwrap();
    let manifest: PlaytestSessionManifest = ron::from_str(&manifest_ron).unwrap();

    assert_eq!(
        manifest.active_control,
        Some(PlaytestActiveControl {
            request: accepted.request,
        })
    );
}

#[test]
fn valid_runtime_request_is_consumed_and_applied() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let session_dir = std::env::temp_dir().join(format!("playtest_runtime_request_{unique}"));
    let transport = FilePlaytestSessionTransport::new(session_dir.clone());
    let request = PlaytestSnapshotRequest {
        extras: payload!(player_velocity_x: 2.5),
    };
    let mut app = PlaytestApp::new(PlaytestLaunchArgs {
        payload_path: "payload.ron".to_string(),
    });
    app.agent_session.attach_transport(transport.clone());

    assert_eq!(
        app.active_snapshot_request,
        PlaytestSnapshotRequest::default()
    );
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        transport.request_path(),
        ron::ser::to_string_pretty(&request, ron::ser::PrettyConfig::default()).unwrap(),
    )
    .unwrap();

    app.poll_runtime_requests();

    assert_eq!(app.active_snapshot_request, request);
    assert!(!transport.request_path().exists());

    let _ = fs::remove_dir_all(session_dir);
}

#[test]
fn manifest_mirrors_latest_accepted_snapshot_request() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let session_dir = std::env::temp_dir().join(format!("playtest_manifest_request_{unique}"));
    let transport = FilePlaytestSessionTransport::new(session_dir.clone());
    let initial_request = PlaytestSnapshotRequest::default();
    let mut app = PlaytestApp::new(PlaytestLaunchArgs {
        payload_path: "payload.ron".to_string(),
    });
    app.agent_session.attach_transport(transport.clone());
    app.active_snapshot_request = initial_request;
    app.agent_session
        .initialize_manifest("payload.ron".to_string());

    let request = PlaytestSnapshotRequest {
        extras: payload!(player_velocity_x: 2.5),
    };

    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        transport.request_path(),
        ron::ser::to_string_pretty(&request, ron::ser::PrettyConfig::default()).unwrap(),
    )
    .unwrap();

    app.poll_runtime_requests();

    let manifest_ron = fs::read_to_string(transport.manifest_path()).unwrap();
    let manifest: PlaytestSessionManifest = ron::from_str(&manifest_ron).unwrap();
    assert_eq!(manifest.snapshot_request, Some(request));

    let _ = fs::remove_dir_all(session_dir);
}

#[test]
fn valid_control_request_is_consumed_and_applied() {
    let _guard = RuntimeControlStateGuard::new();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let session_dir = std::env::temp_dir().join(format!("playtest_control_request_{unique}"));
    let transport = FilePlaytestSessionTransport::new(session_dir.clone());
    let request = PlaytestControlRequest::named(BUILTIN_PROFILE_GROUNDED_WALK_RIGHT);
    let mut app = PlaytestApp::new(PlaytestLaunchArgs {
        payload_path: "payload.ron".to_string(),
    });
    app.agent_session.attach_transport(transport.clone());

    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        transport.control_request_path(),
        ron::ser::to_string_pretty(&request, ron::ser::PrettyConfig::default()).unwrap(),
    )
    .unwrap();

    app.poll_runtime_requests();
    tick_active_playtest_control_timeline();

    assert_eq!(
        get_virtual_input_state().down().get(input_constants::RIGHT),
        Some(&true)
    );
    assert!(!transport.control_request_path().exists());

    let _ = fs::remove_dir_all(session_dir);
}

#[test]
fn manifest_mirrors_latest_accepted_control_request() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let session_dir =
        std::env::temp_dir().join(format!("playtest_manifest_control_request_{unique}"));
    let transport = FilePlaytestSessionTransport::new(session_dir.clone());
    let mut app = PlaytestApp::new(PlaytestLaunchArgs {
        payload_path: "payload.ron".to_string(),
    });
    app.agent_session.attach_transport(transport.clone());
    app.agent_session
        .initialize_manifest("payload.ron".to_string());

    let request = PlaytestControlRequest::named(BUILTIN_PROFILE_GROUNDED_WALK_RIGHT);

    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        transport.control_request_path(),
        ron::ser::to_string_pretty(&request, ron::ser::PrettyConfig::default()).unwrap(),
    )
    .unwrap();

    app.poll_runtime_requests();

    let manifest_ron = fs::read_to_string(transport.manifest_path()).unwrap();
    let manifest: PlaytestSessionManifest = ron::from_str(&manifest_ron).unwrap();
    assert_eq!(
        manifest.active_control,
        Some(PlaytestActiveControl {
            request: request.clone(),
        })
    );

    let _ = fs::remove_dir_all(session_dir);
}

#[test]
fn active_control_snapshot_uses_control_local_frame_index_and_clears_on_completion() {
    let request = PlaytestControlRequest {
        profile: PlaytestControlProfileRef::Inline(PlaytestControlProfile {
            movement_frames: vec![MovementControlFrame {
                frame_count: 1,
                down_inputs: vec![input_constants::RIGHT.to_string()],
                pressed_inputs: Vec::new(),
                released_inputs: Vec::new(),
            }],
            camera_frames: Vec::new(),
        }),
        start_policy: ControlStartPolicy::ReplaceImmediately,
        loop_policy: ControlLoopPolicy::RunOnce,
        chaos: Some(PlaytestChaosConfig { seed: 7 }),
    };
    let mut app = PlaytestApp::new(PlaytestLaunchArgs {
        payload_path: "payload.ron".to_string(),
    });

    app.active_control_runtime = Some(
        ActiveControlSnapshotState::from_accepted(
            accept_playtest_control_request(request.clone()).unwrap(),
        )
        .unwrap(),
    );

    assert_eq!(
        active_control_snapshot_fields(app.active_control_runtime.as_ref(), true),
        (Some("inline".to_string()), Some(7), Some(0))
    );

    advance_active_control_runtime_for_next_snapshot(
        &mut app.active_control_runtime,
        &GameState::Playing,
    );

    assert!(app.active_control_runtime.is_none());
    assert_eq!(
        active_control_snapshot_fields(app.active_control_runtime.as_ref(), false),
        (None, None, None)
    );
}

#[test]
fn unresolved_control_request_keeps_previous_active_control_state() {
    let _guard = RuntimeControlStateGuard::new();
    let valid = PlaytestControlRequest::named(BUILTIN_PROFILE_GROUNDED_WALK_RIGHT);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let session_dir = std::env::temp_dir().join(format!("playtest_control_reject_{unique}"));
    let transport = FilePlaytestSessionTransport::new(session_dir.clone());
    let mut app = PlaytestApp::new(PlaytestLaunchArgs {
        payload_path: "payload.ron".to_string(),
    });
    app.agent_session.attach_transport(transport.clone());

    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        transport.control_request_path(),
        ron::ser::to_string_pretty(&valid, ron::ser::PrettyConfig::default()).unwrap(),
    )
    .unwrap();
    app.poll_runtime_requests();
    fs::write(
        transport.control_request_path(),
        ron::ser::to_string_pretty(
            &PlaytestControlRequest::named("missing_profile"),
            ron::ser::PrettyConfig::default(),
        )
        .unwrap(),
    )
    .unwrap();
    app.poll_runtime_requests();

    assert_eq!(
        app.active_control_request
            .as_ref()
            .map(|accepted| accepted.request.clone()),
        Some(valid.clone())
    );
    assert_eq!(
        app.active_control_runtime
            .as_ref()
            .map(|runtime| runtime.accepted.request.clone()),
        Some(valid)
    );

    let _ = fs::remove_dir_all(session_dir);
}

#[test]
fn malformed_runtime_control_request_file_is_not_deleted() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let session_dir = std::env::temp_dir().join(format!("playtest_bad_control_request_{unique}"));
    let transport = FilePlaytestSessionTransport::new(session_dir.clone());

    fs::create_dir_all(&session_dir).unwrap();
    fs::write(transport.control_request_path(), "not valid ron").unwrap();

    assert!(AgentPlaytestSession::consume_control_runtime_request(&transport).is_none());
    assert!(transport.control_request_path().exists());

    let _ = fs::remove_dir_all(session_dir);
}

#[test]
fn accepted_chaos_request_persists_expanded_control_profile() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let session_dir = std::env::temp_dir().join(format!("playtest_chaos_persist_{unique}"));
    let transport = FilePlaytestSessionTransport::new(session_dir.clone());
    let request = PlaytestControlRequest {
        profile: PlaytestControlProfileRef::Named(BUILTIN_PROFILE_GROUNDED_WALK_RIGHT.to_string()),
        start_policy: ControlStartPolicy::ReplaceImmediately,
        loop_policy: ControlLoopPolicy::RunOnce,
        chaos: Some(PlaytestChaosConfig { seed: 11 }),
    };
    let mut app = PlaytestApp::new(PlaytestLaunchArgs {
        payload_path: "payload.ron".to_string(),
    });
    app.agent_session.attach_transport(transport.clone());

    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        transport.control_request_path(),
        ron::ser::to_string_pretty(&request, ron::ser::PrettyConfig::default()).unwrap(),
    )
    .unwrap();

    app.poll_runtime_requests();

    assert!(transport.expanded_control_path().exists());
    let ron = fs::read_to_string(transport.expanded_control_path()).unwrap();
    let expanded: PlaytestControlProfile = ron::from_str(&ron).unwrap();
    assert!(!expanded.movement_frames.is_empty() || !expanded.camera_frames.is_empty());

    let _ = fs::remove_dir_all(session_dir);
}

#[test]
fn loop_control_request_is_rejected_for_now() {
    let request = PlaytestControlRequest {
        profile: PlaytestControlProfileRef::Named(BUILTIN_PROFILE_GROUNDED_WALK_RIGHT.to_string()),
        start_policy: ControlStartPolicy::ReplaceImmediately,
        loop_policy: ControlLoopPolicy::Loop,
        chaos: None,
    };

    assert!(game_lib::playtest::control::accept_playtest_control_request(request).is_none());
}

#[test]
fn live_camera_state_reflects_runtime_camera_control() {
    let mut manager = CameraManager::default();
    manager.active.camera.target = vec2(10.0, 20.0);
    manager.active.camera.zoom = vec2(1.0, 1.0);

    game_lib::playtest::control::apply_camera_control_frame(
        &mut manager,
        &engine_core::playtest::CameraControlFrame {
            frame_count: 1,
            pan_delta_x: 5.0,
            pan_delta_y: -2.0,
            zoom_delta: 0.25,
            follow_enabled: Some(false),
        },
    );

    assert_eq!(
        live_camera_state(&manager),
        Some(LiveCameraState {
            target: (15.0, 18.0),
            zoom: (1.25, 1.25),
            follow_enabled: false,
            override_active: true,
        })
    );
}

#[test]
fn runtime_payload_serializes_live_camera_fields() {
    let value = engine_core::playtest::payload_value(PlaytestRuntimePayload {
        accumulator_ms: 16.7,
        player_velocity_x: 0.0,
        player_position_x: 0.0,
        player_position_y: 0.0,
        active_control_profile: Some(BUILTIN_PROFILE_CAMERA_PAN_SWEEP.to_string()),
        active_control_seed: None,
        active_control_frame_index: Some(3),
        camera_target: Some((15.0, 18.0)),
        camera_zoom: Some((1.25, 1.25)),
        camera_follow_enabled: Some(false),
        camera_override_active: Some(true),
    })
    .unwrap();

    let ron = ron::to_string(&value).unwrap();

    assert!(ron.contains("camera_target"));
    assert!(ron.contains("camera_zoom"));
    assert!(ron.contains("camera_follow_enabled"));
    assert!(ron.contains("camera_override_active"));
    assert!(ron.contains("15.0"));
    assert!(ron.contains("1.25"));
    assert!(ron.contains("false"));
    assert!(ron.contains("true"));
}

#[test]
fn runtime_payload_does_not_serialize_launch_readiness() {
    let value = engine_core::playtest::payload_value(PlaytestRuntimePayload {
        accumulator_ms: 16.7,
        player_velocity_x: 0.0,
        player_position_x: 0.0,
        player_position_y: 0.0,
        active_control_profile: Some(BUILTIN_PROFILE_CAMERA_PAN_SWEEP.to_string()),
        active_control_seed: None,
        active_control_frame_index: Some(3),
        camera_target: Some((15.0, 18.0)),
        camera_zoom: Some((1.25, 1.25)),
        camera_follow_enabled: Some(false),
        camera_override_active: Some(true),
    })
    .unwrap();

    let ron = ron::to_string(&value).unwrap();

    assert!(!ron.contains("launch_readiness"));
}

#[test]
fn completed_control_clears_snapshot_state_even_when_game_leaves_playing() {
    let request = PlaytestControlRequest {
        profile: PlaytestControlProfileRef::Inline(PlaytestControlProfile {
            movement_frames: vec![MovementControlFrame {
                frame_count: 1,
                down_inputs: vec![input_constants::RIGHT.to_string()],
                pressed_inputs: Vec::new(),
                released_inputs: Vec::new(),
            }],
            camera_frames: Vec::new(),
        }),
        start_policy: ControlStartPolicy::ReplaceImmediately,
        loop_policy: ControlLoopPolicy::RunOnce,
        chaos: None,
    };
    let mut app = PlaytestApp::new(PlaytestLaunchArgs {
        payload_path: "payload.ron".to_string(),
    });

    install_active_playtest_control_timeline(ActiveControlTimeline::new(
        game_lib::playtest::control::resolve_control_profile(&request.profile).unwrap(),
    ));
    tick_active_playtest_control_timeline();
    app.active_control_runtime = Some(
        ActiveControlSnapshotState::from_accepted(
            accept_playtest_control_request(request.clone()).unwrap(),
        )
        .unwrap(),
    );

    advance_active_control_runtime_for_next_snapshot(
        &mut app.active_control_runtime,
        &GameState::Paused,
    );

    assert!(app.active_control_runtime.is_none());
}
