use crate::ecs::{Entity, PointLight};
#[cfg(test)]
use crate::ecs::PointLightLayerScope;
#[cfg(test)]
use crate::worlds::RoomLayer;
use bishop::prelude::Vec2;

pub(crate) type CollectedPointLight<'a> = (Entity, &'a PointLight, Vec2);

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

#[cfg(test)]
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
