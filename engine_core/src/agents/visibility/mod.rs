pub use crate::playtest::session::{
    build_snapshot_payload, merged_snapshot_payload, payload_map_finish, payload_map_insert,
    payload_map_new, payload_value, PlaytestActiveControl as AgentActiveControl,
    PlaytestPayloadError as AgentVisibilityPayloadError,
    PlaytestSessionManifest as AgentSessionManifest, PlaytestSessionRole as AgentSessionRole,
    PlaytestSessionState as AgentSessionState, PlaytestSessionTransport as AgentSessionTransport,
    PlaytestSnapshot as AgentVisibilitySnapshot, PlaytestSnapshotRequest as AgentSnapshotRequest,
};

/// Builds an inline RON payload map from serializable key-value pairs.
#[macro_export]
macro_rules! payload {
    () => {{
        $crate::playtest::payload_map_finish($crate::playtest::payload_map_new())
    }};
    ($($key:ident : $value:expr),* $(,)?) => {{
        let mut map = $crate::playtest::payload_map_new();
        $(
            $crate::playtest::payload_map_insert(
                &mut map,
                stringify!($key).to_string(),
                $crate::playtest::payload_value($value)
                    .expect("payload! values must be serializable"),
            );
        )*
        $crate::playtest::payload_map_finish(map)
    }};
}

#[cfg(test)]
mod tests;
