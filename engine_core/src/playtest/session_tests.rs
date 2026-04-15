use crate::constants::playtest_artifacts;
use crate::playtest::{
    build_snapshot_payload, merged_snapshot_payload, payload_value, PlaytestActiveControl,
    PlaytestControlRequest, PlaytestSessionManifest, PlaytestSessionRole, PlaytestSessionState,
    PlaytestSnapshot, PlaytestSnapshotRequest,
};
use serde::Serialize;

#[derive(Serialize)]
struct SamplePayload {
    accumulator_ms: f32,
    player_velocity_x: f32,
}

#[derive(Serialize)]
enum SampleState {
    Running,
}

#[derive(Serialize)]
struct EnumPayload {
    state: SampleState,
}

fn value_map(entries: &[(&str, ron::Value)]) -> ron::Value {
    let mut map = ron::Map::new();
    for (key, value) in entries {
        map.insert(ron::Value::String((*key).to_string()), value.clone());
    }
    ron::Value::Map(map)
}

#[test]
fn snapshot_request_round_trips_with_extras_only() {
    let extras = value_map(&[("player_velocity_x", ron::Value::Number(4.0.into()))]);
    let request = PlaytestSnapshotRequest {
        extras: extras.clone(),
    };

    let ron = ron::to_string(&request).unwrap();
    let round_trip: PlaytestSnapshotRequest = ron::from_str(&ron).unwrap();

    let ron::Value::Map(round_trip_map) = round_trip.extras else {
        panic!("round-tripped extras should be a map");
    };
    let ron::Value::Map(expected_map) = extras else {
        panic!("expected extras should be a map");
    };

    assert_eq!(round_trip_map.len(), expected_map.len());

    let Some(ron::Value::Number(player_velocity_x)) =
        round_trip_map.get(&ron::Value::String("player_velocity_x".to_string()))
    else {
        panic!("player_velocity_x missing");
    };

    assert!((player_velocity_x.into_f64() - 4.0).abs() < f64::EPSILON);
}

#[test]
fn extras_override_profile_fields_on_collision() {
    let request = PlaytestSnapshotRequest {
        extras: value_map(&[
            ("accumulator_ms", ron::Value::Number(99.0.into())),
            ("custom_flag", ron::Value::Bool(true)),
        ]),
    };

    let payload = merged_snapshot_payload(
        &request,
        value_map(&[
            ("accumulator_ms", ron::Value::Number(16.7.into())),
            ("player_velocity_x", ron::Value::Number(4.0.into())),
        ]),
    );

    let ron::Value::Map(map) = payload else {
        panic!("merged payload should be a map");
    };

    let Some(ron::Value::Number(accumulator_ms)) =
        map.get(&ron::Value::String("accumulator_ms".to_string()))
    else {
        panic!("accumulator_ms missing");
    };
    assert!((accumulator_ms.into_f64() - 99.0).abs() < f64::EPSILON);

    let Some(ron::Value::Number(player_velocity_x)) =
        map.get(&ron::Value::String("player_velocity_x".to_string()))
    else {
        panic!("player_velocity_x missing");
    };
    assert!((player_velocity_x.into_f64() - 4.0).abs() < f64::EPSILON);

    assert_eq!(
        map.get(&ron::Value::String("custom_flag".to_string())),
        Some(&ron::Value::Bool(true))
    );
}

#[test]
fn non_map_extras_do_not_replace_profile_payload() {
    let request = PlaytestSnapshotRequest {
        extras: ron::Value::Bool(true),
    };
    let runtime_payload = value_map(&[
        ("accumulator_ms", ron::Value::Number(16.7.into())),
        ("player_velocity_x", ron::Value::Number(4.0.into())),
    ]);

    assert_eq!(
        build_snapshot_payload(&request, Some(runtime_payload.clone())),
        Some(runtime_payload)
    );

    assert_eq!(build_snapshot_payload(&request, None), None);
}

#[test]
fn build_snapshot_payload_keeps_runtime_payload_when_request_extras_empty() {
    let request = PlaytestSnapshotRequest::default();
    let runtime_payload = Some(value_map(&[(
        "accumulator_ms",
        ron::Value::Number(16.7.into()),
    )]));

    assert_eq!(
        build_snapshot_payload(&request, runtime_payload.clone()),
        runtime_payload
    );
}

#[test]
fn playtest_payload_helper_serializes_typed_data() {
    let value = payload_value(SamplePayload {
        accumulator_ms: 16.7,
        player_velocity_x: 0.0,
    })
    .unwrap();

    let ron = ron::to_string(&value).unwrap();
    assert!(ron.contains("accumulator_ms"));
    assert!(ron.contains("player_velocity_x"));
}

#[test]
fn playtest_payload_helper_supports_enum_fields() {
    let value = payload_value(EnumPayload {
        state: SampleState::Running,
    })
    .unwrap();

    let ron = ron::to_string(&value).unwrap();
    assert!(ron.contains("Running"));
}

#[test]
fn playtest_snapshot_includes_frame_timing_and_session_state() {
    let snapshot = PlaytestSnapshot {
        session_state: PlaytestSessionState::Running,
        frame_time_ms: Some(16.7),
        smoothed_frame_time_ms: Some(14.2),
        mode: Some(playtest_artifacts::PLAYTEST_MODE.to_string()),
        recent_log_count: 3,
        frame_index: Some(0),
        topic: Some(playtest_artifacts::PLAYTEST_RUNTIME_TOPIC.to_string()),
        label: Some(playtest_artifacts::PLAYTEST_FRAME_LABEL.to_string()),
        payload: Some(value_map(&[(
            "frame_time_ms",
            ron::Value::Number(16.7.into()),
        )])),
    };

    let ron = ron::to_string(&snapshot).unwrap();

    assert!(ron.contains("Running"));
    assert!(ron.contains("16.7"));
    assert!(ron.contains("14.2"));
    assert!(ron.contains("runtime"));
}

#[test]
fn session_manifest_round_trips_active_control_metadata() {
    let manifest = PlaytestSessionManifest {
        session_id: "session-1".to_string(),
        role: PlaytestSessionRole::Playtest,
        state: PlaytestSessionState::Running,
        payload_path: Some("/tmp/payload.ron".to_string()),
        snapshot_request: Some(PlaytestSnapshotRequest::default()),
        active_control: Some(PlaytestActiveControl {
            request: PlaytestControlRequest::named("grounded_walk_right"),
        }),
    };

    let ron = ron::to_string(&manifest).unwrap();
    let round_trip: PlaytestSessionManifest = ron::from_str(&ron).unwrap();

    assert_eq!(round_trip, manifest);
}
