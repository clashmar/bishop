use crate::ecs::{Entity, PointLight};
use crate::ecs::PointLightLayerScope;
use crate::worlds::RoomLayer;
use bishop::prelude::{Vec2, Vec3};

pub(crate) type CollectedPointLight<'a> = (Entity, &'a PointLight, Vec2);

/// Prepares room ambient color as a linear RGB multiplier for the ambient pass.
pub(crate) fn ambient_multiplier(darkness: f32, color: Vec3, exposure: f32) -> Vec3 {
    let color = Vec3::new(
        finite_or(color.x, 1.0).max(0.0),
        finite_or(color.y, 1.0).max(0.0),
        finite_or(color.z, 1.0).max(0.0),
    );
    let exposure = finite_or(exposure, 1.0).max(0.0);
    let darkness = finite_or(darkness, 0.0).clamp(0.0, 1.0);
    color * exposure * (1.0 - darkness)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

/// Returns normalized radial falloff for a point light.
pub(crate) fn point_light_attenuation(distance: f32, radius: f32, exponent: f32) -> f32 {
    if !distance.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return 0.0;
    }

    let exponent = finite_or(exponent, 1.0).max(0.0);
    (1.0 - distance.max(0.0) / radius)
        .clamp(0.0, 1.0)
        .powf(exponent)
}

/// Point lights collected for one room layer and depth group.
#[derive(Default)]
pub(crate) struct RoomLayerLighting<'a> {
    /// Point lights attached to entities in this layer and depth group.
    pub(crate) point_lights: Vec<CollectedPointLight<'a>>,
}

impl<'a> RoomLayerLighting<'a> {
    /// Adds an entity-relative point light using the entity's interpolated position.
    pub(crate) fn collect_point_light(
        &mut self,
        entity: Entity,
        light: &'a PointLight,
        interpolated_position: Vec2,
    ) {
        self.point_lights.push((
            entity,
            light,
            interpolated_position + light.offset,
        ));
    }

    /// Sorts collected lights by their owning entity ID.
    pub(crate) fn sort(&mut self) {
        self.point_lights.sort_by_key(|light| light.0);
    }
}

pub(crate) fn point_light_affects_layer(
    light: &PointLight,
    owner_layer: RoomLayer,
    target_layer: RoomLayer,
    visible_layers: &[RoomLayer],
) -> bool {
    if !visible_layers.contains(&target_layer) {
        return false;
    }

    match light.layer_scope {
        PointLightLayerScope::OwningLayer => target_layer == owner_layer,
        PointLightLayerScope::AllVisibleLayers => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{ambient_multiplier, point_light_affects_layer, point_light_attenuation};
    use crate::ecs::{PointLight, PointLightLayerScope};
    use crate::worlds::RoomLayer;
    use bishop::prelude::vec3;

    #[test]
    fn default_ambient_settings_produce_neutral_output() {
        let multiplier = ambient_multiplier(0.0, vec3(1.0, 1.0, 1.0), 1.0);

        assert_eq!(multiplier, vec3(1.0, 1.0, 1.0));
    }

    #[test]
    fn maximum_darkness_produces_zero_ambient_output() {
        let multiplier = ambient_multiplier(1.0, vec3(0.6, 0.8, 0.4), 2.0);

        assert_eq!(multiplier, vec3(0.0, 0.0, 0.0));
    }

    #[test]
    fn non_finite_ambient_values_fall_back_to_neutral_inputs() {
        let multiplier = ambient_multiplier(
            f32::NAN,
            vec3(f32::INFINITY, 0.5, f32::NAN),
            f32::INFINITY,
        );

        assert_eq!(multiplier, vec3(1.0, 0.5, 1.0));
    }

    #[test]
    fn point_light_scope_only_selects_visible_layers() {
        let visible = [RoomLayer::Front, RoomLayer::Back];
        let light = PointLight::default();

        assert!(point_light_affects_layer(
            &light,
            RoomLayer::Back,
            RoomLayer::Back,
            &visible,
        ));
        assert!(!point_light_affects_layer(
            &light,
            RoomLayer::Back,
            RoomLayer::Front,
            &visible,
        ));

        let cross_layer_light = PointLight {
            layer_scope: PointLightLayerScope::AllVisibleLayers,
            ..light
        };
        assert!(point_light_affects_layer(
            &cross_layer_light,
            RoomLayer::Back,
            RoomLayer::Front,
            &visible,
        ));
        assert!(!point_light_affects_layer(
            &cross_layer_light,
            RoomLayer::Back,
            RoomLayer::Front,
            &[RoomLayer::Back],
        ));
    }

    #[test]
    fn point_light_attenuation_falls_to_zero_at_its_radius() {
        assert!(point_light_attenuation(31.0, 32.0, 2.0) > 0.0);
        assert!(point_light_attenuation(31.999, 32.0, 2.0) < 0.001);
        assert_eq!(point_light_attenuation(32.0, 32.0, 2.0), 0.0);
        assert_eq!(point_light_attenuation(33.0, 32.0, 2.0), 0.0);
    }
}
