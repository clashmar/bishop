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

impl LayerCompositionMode {
    pub const ALL: [Self; 2] = [Self::Hidden, Self::DollsHouse];

    pub fn ui_label(self) -> &'static str {
        match self {
            Self::Hidden => "Hidden",
            Self::DollsHouse => "Dolls House",
        }
    }
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
    /// Returns the authored back-layer bounds, or full-room fallback when no zones exist.
    pub fn effective_back_bounds(&self, room_bounds: Rect) -> Vec<Rect> {
        match &self.back {
            Some(back) if !back.interior_zones.is_empty() => {
                back.interior_zones.iter().map(|zone| zone.bounds).collect()
            }
            Some(_) | None => vec![room_bounds],
        }
    }

    /// Returns the currently active back-layer bounds for one viewpoint.
    pub fn active_back_bounds(
        &self,
        room_bounds: Rect,
        current_layer: RoomLayer,
        viewpoint_position: Option<Vec2>,
    ) -> Vec<Rect> {
        if current_layer != RoomLayer::Back {
            return vec![];
        }

        let Some(back) = &self.back else {
            return vec![];
        };

        if back.interior_zones.is_empty() {
            return vec![room_bounds];
        }

        let Some(viewpoint_position) = viewpoint_position else {
            return vec![];
        };

        back.interior_zones
            .iter()
            .filter(|zone| zone.bounds.contains(viewpoint_position))
            .map(|zone| zone.bounds)
            .collect()
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

    #[test]
    fn interior_zones_round_trip_with_stable_ids() {
        let zones = vec![
            InteriorZone {
                id: InteriorZoneId(3),
                bounds: Rect::new(0.0, 0.0, 32.0, 32.0),
            },
            InteriorZone {
                id: InteriorZoneId(9),
                bounds: Rect::new(32.0, 0.0, 32.0, 32.0),
            },
        ];
        let variant = RoomVariant {
            id: "default".to_string(),
            layers: RoomLayers {
                back: Some(BackRoomLayer {
                    composition_mode: LayerCompositionMode::Hidden,
                    interior_zones: zones.clone(),
                }),
            },
            ..Default::default()
        };

        let ron = ron::ser::to_string_pretty(&variant, ron::ser::PrettyConfig::new()).unwrap();
        let parsed: RoomVariant = ron::from_str(&ron).unwrap();

        let back = parsed.layers.back.unwrap();
        assert_eq!(back.interior_zones, zones);
    }

    #[test]
    fn active_back_bounds_when_back_has_no_zones_then_uses_room_bounds() {
        let room_bounds = Rect::new(0.0, 0.0, 128.0, 128.0);
        let layers = RoomLayers {
            back: Some(BackRoomLayer::default()),
        };

        assert_eq!(
            layers.active_back_bounds(room_bounds, RoomLayer::Back, Some(Vec2::new(8.0, 8.0))),
            vec![room_bounds],
        );
    }

    #[test]
    fn active_back_bounds_when_zones_exist_then_returns_only_matching_zones() {
        let room_bounds = Rect::new(0.0, 0.0, 128.0, 128.0);
        let zone_a = InteriorZone {
            id: InteriorZoneId(1),
            bounds: Rect::new(0.0, 0.0, 32.0, 32.0),
        };
        let zone_b = InteriorZone {
            id: InteriorZoneId(2),
            bounds: Rect::new(64.0, 0.0, 32.0, 32.0),
        };
        let layers = RoomLayers {
            back: Some(BackRoomLayer {
                composition_mode: LayerCompositionMode::Hidden,
                interior_zones: vec![zone_a, zone_b],
            }),
        };

        assert_eq!(
            layers.active_back_bounds(room_bounds, RoomLayer::Back, Some(Vec2::new(8.0, 8.0))),
            vec![zone_a.bounds],
        );
        assert!(layers
            .active_back_bounds(room_bounds, RoomLayer::Front, Some(Vec2::new(8.0, 8.0)))
            .is_empty());
        assert!(layers
            .active_back_bounds(room_bounds, RoomLayer::Back, Some(Vec2::new(48.0, 8.0)))
            .is_empty());
    }
}
