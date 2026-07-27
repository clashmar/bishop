use bishop::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomLayer {
    #[default]
    Front,
    Back,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerCompositionMode {
    #[default]
    Hidden,
    DollsHouse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InteriorZoneId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InteriorZone {
    pub id: InteriorZoneId,
    pub bounds: Rect,
}

impl Default for InteriorZone {
    fn default() -> Self {
        Self {
            id: InteriorZoneId::default(),
            bounds: Rect::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackRoomLayer {
    pub composition_mode: LayerCompositionMode,
    pub interior_zones: Vec<InteriorZone>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoomLayers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub back: Option<BackRoomLayer>,
}

impl RoomLayers {
    pub fn effective_back_bounds(&self, room_bounds: Rect) -> Vec<Rect> {
        match &self.back {
            Some(back) if !back.interior_zones.is_empty() => {
                back.interior_zones.iter().map(|zone| zone.bounds).collect()
            }
            Some(_) | None => vec![room_bounds],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::room::RoomVariant;

    #[test]
    fn room_without_back_layer_round_trips_through_ron() {
        let variant = RoomVariant {
            id: "default".to_string(),
            layers: RoomLayers::default(),
            ..Default::default()
        };

        let ron = ron::ser::to_string_pretty(&variant, ron::ser::PrettyConfig::new()).unwrap();
        let parsed: RoomVariant = ron::from_str(&ron).unwrap();

        assert_eq!(parsed.layers, RoomLayers::default());
    }

    #[test]
    fn room_with_back_layer_round_trips_through_ron() {
        let zone = InteriorZone {
            id: InteriorZoneId(7),
            bounds: Rect::new(16.0, 32.0, 48.0, 64.0),
        };
        let variant = RoomVariant {
            id: "default".to_string(),
            layers: RoomLayers {
                back: Some(BackRoomLayer {
                    composition_mode: LayerCompositionMode::DollsHouse,
                    interior_zones: vec![zone],
                }),
            },
            ..Default::default()
        };

        let ron = ron::ser::to_string_pretty(&variant, ron::ser::PrettyConfig::new()).unwrap();
        let parsed: RoomVariant = ron::from_str(&ron).unwrap();

        let back = parsed.layers.back.unwrap();
        assert_eq!(back.composition_mode, LayerCompositionMode::DollsHouse);
        let parsed_zone = back.interior_zones[0];
        assert_eq!(parsed_zone.id, InteriorZoneId(7));
        assert_eq!(parsed_zone.bounds, zone.bounds);
    }
}
