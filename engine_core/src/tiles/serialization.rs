use crate::tiles::tile::TileDefId;
use serde::{Deserialize, Deserializer, Serializer};
use std::collections::HashMap;

pub fn serialize_tiles<S: Serializer>(
    tiles: &HashMap<(usize, usize), TileDefId>,
    s: S,
) -> Result<S::Ok, S::Error> {
    use std::fmt::Write as _;
    let mut out = String::new();
    let mut ordered_tiles: Vec<_> = tiles.iter().collect();
    ordered_tiles.sort_by_key(|((x, y), _)| (*y, *x));

    for ((x, y), TileDefId(id)) in ordered_tiles {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TileWrapper {
        #[serde(serialize_with = "serialize_tiles")]
        tiles: HashMap<(usize, usize), TileDefId>,
    }

    #[test]
    fn serialize_tiles_orders_entries_by_row_then_column() {
        let tiles = HashMap::from([
            ((3, 2), TileDefId(9)),
            ((1, 0), TileDefId(4)),
            ((0, 0), TileDefId(2)),
            ((2, 1), TileDefId(7)),
        ]);

        let wrapper = TileWrapper { tiles };
        let ron = ron::to_string(&wrapper).expect("serialize tile wrapper");

        assert_eq!(ron, "(tiles:\"0,0,2;1,0,4;2,1,7;3,2,9;\")");
    }
}
