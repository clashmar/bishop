use crate::ecs::reflect_field::{FieldInfo, Reflect, ReflectField};
use crate::ecs::{PointLightLayerScope, PointLightStyle};
use bishop::prelude::{Vec3, vec3};
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Controls sprite-shaped light emission for an entity.
#[ecs_component]
#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default, rename_all = "snake_case")]
pub struct Glow {
    /// RGB emission color.
    #[serde_as(as = "serde_with::FromInto<[f32; 3]>")]
    pub color: Vec3,
    /// Emitted radiance multiplier.
    pub intensity: f32,
    /// Halo spread in world units.
    pub halo_spread: f32,
    /// Layers this emission can illuminate.
    pub layer_scope: PointLightLayerScope,
    /// Emission sampling style.
    pub style: PointLightStyle,
    /// World-space pixel-grid size when using Pixel style.
    pub pixel_size: i32,
}

impl Reflect for Glow {
    fn fields(&mut self) -> Vec<FieldInfo<'_>> {
        vec![
            Vec3::field_info(&mut self.color, "color"),
            f32::field_info(&mut self.intensity, "intensity"),
            f32::field_info(&mut self.halo_spread, "halo_spread"),
            i32::field_info(&mut self.pixel_size, "pixel_size"),
        ]
    }
}

impl Glow {
    /// Clamps authored numeric settings to the supported runtime ranges.
    pub fn normalize_settings(&mut self) {
        self.color = vec3(
            normalize_float(self.color.x, 0.0, 1.0, 1.0),
            normalize_float(self.color.y, 0.0, 1.0, 1.0),
            normalize_float(self.color.z, 0.0, 1.0, 1.0),
        );
        self.intensity = normalize_float(self.intensity, 0.0, 20.0, 1.0);
        self.halo_spread = normalize_float(self.halo_spread, 0.0, 256.0, 16.0);
        self.pixel_size = self.pixel_size.clamp(1, 16);
    }
}

fn normalize_float(value: f32, min: f32, max: f32, default: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        default
    }
}

impl Default for Glow {
    fn default() -> Self {
        Self {
            color: vec3(1.0, 1.0, 1.0),
            intensity: 1.0,
            halo_spread: 16.0,
            layer_scope: PointLightLayerScope::default(),
            style: PointLightStyle::default(),
            pixel_size: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Glow;
    use crate::ecs::{PointLightLayerScope, PointLightStyle};
    use bishop::prelude::vec3;

    #[test]
    fn glow_default_uses_neutral_emission_settings() {
        let glow = Glow::default();

        assert_eq!(glow.color, vec3(1.0, 1.0, 1.0));
        assert_eq!(glow.intensity, 1.0);
        assert_eq!(glow.halo_spread, 16.0);
        assert_eq!(glow.layer_scope, PointLightLayerScope::OwningLayer);
        assert_eq!(glow.style, PointLightStyle::Smooth);
        assert_eq!(glow.pixel_size, 1);
    }

    #[test]
    fn glow_round_trips_authored_settings() -> Result<(), ron::Error> {
        let glow = Glow {
            color: vec3(0.2, 0.5, 0.9),
            intensity: 3.5,
            halo_spread: 40.0,
            layer_scope: PointLightLayerScope::AllVisibleLayers,
            style: PointLightStyle::Pixel,
            pixel_size: 4,
        };

        let serialized = ron::to_string(&glow)?;
        let restored: Glow = ron::from_str(&serialized)?;

        assert_eq!(restored.color, glow.color);
        assert_eq!(restored.intensity, glow.intensity);
        assert_eq!(restored.halo_spread, glow.halo_spread);
        assert_eq!(restored.layer_scope, glow.layer_scope);
        assert_eq!(restored.style, glow.style);
        assert_eq!(restored.pixel_size, glow.pixel_size);
        Ok(())
    }

    #[test]
    fn glow_normalizes_numeric_settings_at_runtime_boundary() {
        let mut glow = Glow {
            color: vec3(-0.5, 1.5, f32::NAN),
            intensity: 30.0,
            halo_spread: f32::NAN,
            pixel_size: 20,
            ..Default::default()
        };

        glow.normalize_settings();

        assert_eq!(glow.color, vec3(0.0, 1.0, 1.0));
        assert_eq!(glow.intensity, 20.0);
        assert_eq!(glow.halo_spread, 16.0);
        assert_eq!(glow.pixel_size, 16);
    }
}
