use crate::worlds::WorldId;
use serde::{Deserialize, Serialize};

/// Lightweight game-data manifest that replaces the monolithic `Game` blob in `game.ron`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GameDataManifest {
    pub version: u32,
    pub name: String,
    pub current_world_id: Option<WorldId>,
    pub world_ids: Vec<WorldId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_data_manifest_round_trips_without_inline_worlds() {
        let manifest = GameDataManifest {
            version: 1,
            name: "Demo".to_string(),
            current_world_id: Some(WorldId(1)),
            world_ids: vec![WorldId(1)],
        };

        let ron = ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::new()).unwrap();
        let parsed: GameDataManifest = ron::from_str(&ron).unwrap();

        assert_eq!(parsed.world_ids, manifest.world_ids);
    }
}
