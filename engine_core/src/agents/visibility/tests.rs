use crate::agents::AgentPlaytestControlRequest;
use crate::constants::agents;
use crate::playtest::{
    build_snapshot_payload, merged_snapshot_payload, payload_value, PlaytestActiveControl,
    PlaytestSessionManifest, PlaytestSessionRole, PlaytestSessionState, PlaytestSnapshot,
    PlaytestSnapshotRequest,
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

mod macro_hygiene {
    mod ron {}

    #[test]
    fn payload_macro_ignores_call_site_ron_module() {
        let value = crate::payload!(flag: true);

        assert_eq!(
            value,
            ::ron::Value::Map(
                [(
                    ::ron::Value::String("flag".to_string()),
                    ::ron::Value::Bool(true),
                )]
                .into_iter()
                .collect(),
            )
        );
    }
}

#[derive(Serialize)]
struct EnumPayload {
    state: SampleState,
}

#[test]
fn payload_macro_builds_inline_map_without_named_payload_type() {
    let value = payload!(
        accumulator_ms: 16.7,
        state: SampleState::Running,
    );

    let ron::Value::Map(map) = &value else {
        panic!("payload! should build a map");
    };

    let Some(ron::Value::Number(number)) =
        map.get(&ron::Value::String("accumulator_ms".to_string()))
    else {
        panic!("accumulator_ms should serialize as a number");
    };
    assert!((number.into_f64() - 16.7).abs() < f64::EPSILON);
    assert_eq!(
        map.get(&ron::Value::String("state".to_string())),
        Some(&ron::Value::String("Running".to_string()))
    );

    let ron = ron::to_string(&value).unwrap();
    assert!(ron.contains("accumulator_ms"));
    assert!(ron.contains("Running"));
}

#[test]
fn snapshot_request_round_trips_with_extras_only() {
    let request = PlaytestSnapshotRequest {
        extras: payload!(player_velocity_x: 4.0),
    };

    let ron = ron::to_string(&request).unwrap();
    let round_trip: PlaytestSnapshotRequest = ron::from_str(&ron).unwrap();

    assert_eq!(round_trip.extras, payload!(player_velocity_x: 4.0));
}

#[test]
fn extras_override_profile_fields_on_collision() {
    let request = PlaytestSnapshotRequest {
        extras: payload!(
            accumulator_ms: 99.0,
            custom_flag: true,
        ),
    };

    let payload = merged_snapshot_payload(
        &request,
        payload!(
            accumulator_ms: 16.7,
            player_velocity_x: 4.0,
        ),
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

    let payload = build_snapshot_payload(
        &request,
        Some(payload!(
            accumulator_ms: 16.7,
            player_velocity_x: 4.0,
        )),
    );

    assert_eq!(
        payload,
        Some(payload!(
            accumulator_ms: 16.7,
            player_velocity_x: 4.0,
        ))
    );

    assert_eq!(build_snapshot_payload(&request, None), None);
}

#[test]
fn build_snapshot_payload_keeps_runtime_payload_when_request_extras_empty() {
    let request = PlaytestSnapshotRequest::default();
    let runtime_payload = Some(payload!(accumulator_ms: 16.7));

    assert_eq!(
        build_snapshot_payload(&request, runtime_payload.clone()),
        runtime_payload
    );
}

#[test]
fn agent_visibility_payload_helper_serializes_typed_data() {
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
fn agent_visibility_payload_helper_supports_enum_fields() {
    let value = payload_value(EnumPayload {
        state: SampleState::Running,
    })
    .unwrap();

    let ron = ron::to_string(&value).unwrap();
    assert!(ron.contains("Running"));
}

#[test]
fn agent_visibility_snapshot_includes_frame_timing_and_session_state() {
    let snapshot = PlaytestSnapshot {
        session_state: PlaytestSessionState::Running,
        frame_time_ms: Some(16.7),
        smoothed_frame_time_ms: Some(14.2),
        mode: Some(agents::PLAYTEST_MODE.to_string()),
        recent_log_count: 3,
        frame_index: Some(0),
        topic: Some(agents::PLAYTEST_RUNTIME_TOPIC.to_string()),
        label: Some(agents::PLAYTEST_FRAME_LABEL.to_string()),
        payload: Some(
            [("frame_time_ms", ron::Value::from(16.7))]
                .into_iter()
                .collect(),
        ),
    };

    let ron = match ron::to_string(&snapshot) {
        Ok(ron) => ron,
        Err(err) => panic!("failed to serialize snapshot: {err}"),
    };

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
        snapshot_request: Some(PlaytestSnapshotRequest { extras: payload!() }),
        active_control: Some(PlaytestActiveControl {
            request: AgentPlaytestControlRequest::named("grounded_walk_right"),
        }),
    };

    let ron = ron::to_string(&manifest).unwrap();
    let round_trip: PlaytestSessionManifest = ron::from_str(&ron).unwrap();

    assert_eq!(round_trip, manifest);
}
