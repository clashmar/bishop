//engine_core/src/game/game_map.rs
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Serialize, Deserialize, Default)]
pub struct GameMap {}
