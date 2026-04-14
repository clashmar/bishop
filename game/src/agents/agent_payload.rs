use engine_core::agents::payload::AgentBuiltPayload;
use ron::de::from_str;
use std::fs;
use std::path::Path;

/// Loads an agent-assembled payload from disk.
pub fn load_agent_payload(path: impl AsRef<Path>) -> Result<AgentBuiltPayload, String> {
    let path = path.as_ref();
    let payload_ron = fs::read_to_string(path)
        .map_err(|error| format!("Could not read agent payload '{}': {error}", path.display()))?;

    from_str(&payload_ron).map_err(|error| {
        format!(
            "Failed to deserialize agent payload '{}': {error}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::load_agent_payload;
    use engine_core::agents::payload::AgentPayloadSpec;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_agent_payload_round_trips_built_payload() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("agent_payload_{unique}.ron"));

        let built = AgentPayloadSpec::synthetic("TestGame")
            .add_room("HeadlessRoom")
            .add_entity("Player")
            .attach_component(
                "Player",
                "Transform",
                "(visible:true, position:(x:0.0,y:0.0), pivot:())",
            )
            .build()
            .unwrap();

        fs::write(
            &path,
            ron::ser::to_string_pretty(&built, ron::ser::PrettyConfig::default()).unwrap(),
        )
        .unwrap();

        let loaded = load_agent_payload(&path).unwrap();
        assert_eq!(loaded.game.name, "TestGame");
        assert_eq!(loaded.room.name, "HeadlessRoom");

        let _ = fs::remove_file(path);
    }
}
