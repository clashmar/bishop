use crate::ecs::reflect_field::{FieldInfo, Reflect, ReflectField};
use crate::inspector_module;
use bishop::prelude::{vec2, vec3, Vec2, Vec3};
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Layers a point light can illuminate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointLightLayerScope {
    /// Only the layer that owns the light entity.
    #[default]
    OwningLayer,
    /// Every visible layer in the room.
    AllVisibleLayers,
}

/// Sampling style used when rendering a point light.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointLightStyle {
    /// Smoothly sample the light falloff.
    #[default]
    Smooth,
    /// Quantize the light sampling to a pixel grid.
    Pixel,
}

/// An entity-relative point light source.
#[ecs_component]
#[serde_as]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PointLight {
    /// Position offset from the owning entity.
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub offset: Vec2,
    /// RGB light color.
    #[serde_as(as = "serde_with::FromInto<[f32; 3]>")]
    pub color: Vec3,
    /// Light intensity multiplier.
    pub intensity: f32,
    /// Light radius in world units.
    pub radius: f32,
    /// Falloff exponent.
    pub attenuation: f32,
    /// Layers this light can illuminate.
    pub layer_scope: PointLightLayerScope,
    /// Light sampling style.
    pub style: PointLightStyle,
    /// Quantization size for pixel-style sampling.
    pub pixel_size: i32,
}

inspector_module!(
    PointLight,
    removable = true,
    title = <PointLight>::TYPE_NAME,
    tooltip = "Adds an entity-relative point light."
);

impl Reflect for PointLight {
    fn fields(&mut self) -> Vec<FieldInfo<'_>> {
        vec![
            Vec2::field_info(&mut self.offset, "offset"),
            Vec3::field_info(&mut self.color, "color"),
            f32::field_info(&mut self.intensity, "intensity"),
            f32::field_info(&mut self.radius, "radius"),
            f32::field_info(&mut self.attenuation, "attenuation"),
            i32::field_info(&mut self.pixel_size, "pixel_size"),
        ]
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            offset: vec2(0.0, 0.0),
            color: vec3(1.0, 1.0, 1.0),
            intensity: 1.0,
            radius: 64.0,
            attenuation: 1.0,
            layer_scope: PointLightLayerScope::default(),
            style: PointLightStyle::default(),
            pixel_size: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PointLight, PointLightLayerScope, PointLightStyle};
    use crate::ecs::component_registry::COMPONENTS;
    use crate::ecs::comp_type_name;
    use bishop::prelude::{vec2, vec3};

    #[test]
    fn point_light_default_uses_neutral_settings() {
        let light = PointLight::default();

        assert_eq!(light.offset, vec2(0.0, 0.0));
        assert_eq!(light.color, vec3(1.0, 1.0, 1.0));
        assert_eq!(light.intensity, 1.0);
        assert_eq!(light.radius, 64.0);
        assert_eq!(light.attenuation, 1.0);
        assert_eq!(light.layer_scope, PointLightLayerScope::OwningLayer);
        assert_eq!(light.style, PointLightStyle::Smooth);
        assert_eq!(light.pixel_size, 1);
    }

    #[test]
    fn point_light_round_trips_all_authored_settings() -> Result<(), ron::Error> {
        let light = PointLight {
            offset: vec2(2.5, -3.0),
            color: vec3(0.2, 0.5, 0.9),
            intensity: 1.7,
            radius: 120.0,
            attenuation: 2.25,
            layer_scope: PointLightLayerScope::AllVisibleLayers,
            style: PointLightStyle::Pixel,
            pixel_size: 4,
        };

        let serialized = ron::to_string(&light)?;
        let restored: PointLight = ron::from_str(&serialized)?;

        assert_eq!(restored.offset, light.offset);
        assert_eq!(restored.color, light.color);
        assert_eq!(restored.intensity, light.intensity);
        assert_eq!(restored.radius, light.radius);
        assert_eq!(restored.attenuation, light.attenuation);
        assert_eq!(restored.layer_scope, light.layer_scope);
        assert_eq!(restored.style, light.style);
        assert_eq!(restored.pixel_size, light.pixel_size);
        Ok(())
    }

    #[test]
    fn point_light_is_registered_as_an_ecs_component() {
        assert!(COMPONENTS
            .iter()
            .any(|component| component.type_name == comp_type_name::<PointLight>()));
    }
}
