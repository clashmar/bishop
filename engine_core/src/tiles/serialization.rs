use crate::tiles::tile::TileDefId;
use serde::{Deserialize, Deserializer, Serializer};
use std::collections::HashMap;

pub fn serialize_tiles<S: Serializer>(
    tiles: &HashMap<(usize, usize), TileDefId>,
    s: S,
) -> Result<S::Ok, S::Error> {
    use std::fmt::Write as _;
    let mut out = String::new();
    for ((x, y), TileDefId(id)) in tiles {
        write!(out, "{x},{y},{id};").map_err(serde::ser::Error::custom)?;
    }
    s.serialize_str(&out)
}

pub fn deserialize_tiles<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<HashMap<(usize, usize), TileDefId>, D::Error> {
    let s = String::deserialize(d)?;
    let mut map = HashMap::new();
    for entry in s.split(';').filter(|e| !e.is_empty()) {
        let mut parts = entry.splitn(3, ',');
        let x = parts.next().and_then(|v| v.parse().ok());
        let y = parts.next().and_then(|v| v.parse().ok());
        let id = parts.next().and_then(|v| v.parse().ok());
        match (x, y, id) {
            (Some(x), Some(y), Some(id)) => {
                map.insert((x, y), TileDefId(id));
            }
            _ => {
                return Err(serde::de::Error::custom(format!(
                    "Invalid tile entry: '{entry}'"
                )));
            }
        }
    }
    Ok(map)
}
