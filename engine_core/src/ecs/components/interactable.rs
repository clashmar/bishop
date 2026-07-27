use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::{CurrentRoom, Transform};
use crate::inspector_module;
use bishop::prelude::{Rect, Vec2};
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Semantic interactable area kind used by helper APIs and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractableShape {
    Circle,
    Rect,
}

/// Component for interactable entities.
#[ecs_component]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct Interactable {
    /// Maximum interaction distance.
    pub range: f32,
    /// When true, the authored interactable area is rectangular instead of circular.
    pub use_rect: bool,
    /// Local offset from the entity origin to the interactable area's center.
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub offset: Vec2,
    /// Radius used when `use_rect == false`.
    pub radius: f32,
    /// Rect size used when `use_rect == true`.
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub rect_size: Vec2,
    // TODO: Add priority,
    // enabled/disabled,
    // prompt,
    // facing
    // event dispatch
}
inspector_module!(Interactable);

impl Default for Interactable {
    fn default() -> Self {
        Self {
            range: 20.0,
            use_rect: false,
            offset: Vec2::ZERO,
            radius: 8.0,
            rect_size: Vec2::splat(16.0),
        }
    }
}

impl Interactable {
    /// Creates a circular interactable area.
    pub fn circle(range: f32, offset: Vec2, radius: f32) -> Self {
        Self {
            range,
            use_rect: false,
            offset,
            radius,
            ..Default::default()
        }
    }

    /// Creates a rectangular interactable area.
    pub fn rect(range: f32, offset: Vec2, rect_size: Vec2) -> Self {
        Self {
            range,
            use_rect: true,
            offset,
            rect_size,
            ..Default::default()
        }
    }

    /// Returns the semantic shape represented by this component.
    pub fn shape(&self) -> InteractableShape {
        if self.use_rect {
            InteractableShape::Rect
        } else {
            InteractableShape::Circle
        }
    }

    /// Returns the world-space bounds enclosing this interactable area.
    pub fn bounds_at(&self, origin: Vec2) -> Rect {
        match self.shape() {
            InteractableShape::Circle => Rect::new(
                origin.x + self.offset.x - self.radius,
                origin.y + self.offset.y - self.radius,
                self.radius * 2.0,
                self.radius * 2.0,
            ),
            InteractableShape::Rect => Rect::new(
                origin.x + self.offset.x - self.rect_size.x * 0.5,
                origin.y + self.offset.y - self.rect_size.y * 0.5,
                self.rect_size.x,
                self.rect_size.y,
            ),
        }
    }

    /// Returns true when this interactable area is fully contained in the supplied rect union.
    pub fn area_contained_in_bounds(&self, origin: Vec2, bounds: &[Rect]) -> bool {
        let area_bounds = self.bounds_at(origin);
        match self.shape() {
            InteractableShape::Rect => area_bounds.contained_in_union(bounds),
            InteractableShape::Circle => {
                let center = origin + self.offset;
                let checkpoints = [
                    center,
                    center + Vec2::new(self.radius, 0.0),
                    center + Vec2::new(-self.radius, 0.0),
                    center + Vec2::new(0.0, self.radius),
                    center + Vec2::new(0.0, -self.radius),
                ];
                checkpoints
                    .iter()
                    .all(|point| Rect::point_in_union(*point, bounds))
                    && area_bounds.contained_in_union(bounds)
            }
        }
    }
}

/// Returns the best interactable entity candidate for the player in the current room/layer.
pub fn find_best_interactable(ecs: &Ecs) -> Option<Entity> {
    let player = ecs.get_player_entity()?;
    let player_pos = ecs.get_player_transform()?.position;
    let player_room = ecs.get::<CurrentRoom>(player).copied()?;

    find_best_interactable_in_layer(ecs, player_room.room_id, player_room.layer, player_pos)
}

/// Returns the closest interactable entity candidate inside one room/layer pair.
pub fn find_best_interactable_in_layer(
    ecs: &Ecs,
    room_id: crate::worlds::RoomId,
    layer: crate::worlds::RoomLayer,
    source_pos: Vec2,
) -> Option<Entity> {
    let interactables = ecs.get_store::<Interactable>();
    let positions = ecs.get_store::<Transform>();

    let mut best: Option<(Entity, f32)> = None;

    for &entity in ecs.entities_in_room_layer(room_id, layer) {
        ecs.assert_room_membership(room_id, entity);

        let Some(interactable) = interactables.get(entity) else {
            continue;
        };

        let Some(pos) = positions.get(entity).map(|transform| transform.position) else {
            continue;
        };

        let dist = source_pos.distance(pos);
        if dist > interactable.range {
            continue;
        }

        match best {
            None => best = Some((entity, dist)),
            Some((_, best_dist)) if dist < best_dist => best = Some((entity, dist)),
            _ => {}
        }
    }

    best.map(|(entity, _)| entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::{RoomId, RoomLayer};

    #[test]
    fn find_best_interactable_ignores_other_layers_in_the_same_room() {
        let mut ecs = Ecs::default();
        let room_id = RoomId(3);

        let front_entity = ecs.create_entity()
            .with(Transform {
                position: Vec2::new(8.0, 0.0),
                ..Default::default()
            })
            .with(Interactable {
                range: 100.0,
                ..Default::default()
            })
            .with_current_room(room_id)
            .finish();

        let player = ecs.create_entity()
            .with(Transform {
                position: Vec2::new(0.0, 0.0),
                ..Default::default()
            })
            .with(crate::ecs::Player::default())
            .with_current_room(room_id)
            .finish();

        ecs.create_entity()
            .with(Transform {
                position: Vec2::new(1.0, 0.0),
                ..Default::default()
            })
            .with(Interactable {
                range: 100.0,
                ..Default::default()
            })
            .with_current_room_layer(room_id, RoomLayer::Back)
            .finish();

        assert_eq!(ecs.get_player_entity(), Some(player));
        assert_eq!(find_best_interactable(&ecs), Some(front_entity));
    }

    #[test]
    fn find_best_interactable_ignores_other_rooms() {
        let mut ecs = Ecs::default();
        let player_room = RoomId(3);
        let other_room = RoomId(4);

        let player = ecs.create_entity()
            .with(Transform {
                position: Vec2::new(0.0, 0.0),
                ..Default::default()
            })
            .with(crate::ecs::Player::default())
            .with_current_room(player_room)
            .finish();

        ecs.create_entity()
            .with(Transform {
                position: Vec2::new(1.0, 0.0),
                ..Default::default()
            })
            .with(Interactable {
                range: 100.0,
                ..Default::default()
            })
            .with_current_room(other_room)
            .finish();

        assert_eq!(ecs.get_player_entity(), Some(player));
        assert_eq!(find_best_interactable(&ecs), None);
    }

    #[test]
    fn interactable_area_contained_in_zone_union_when_rect_then_validation_passes() {
        let interactable = Interactable::rect(100.0, Vec2::ZERO, Vec2::new(16.0, 16.0));

        assert!(interactable.area_contained_in_bounds(
            Vec2::new(8.0, 8.0),
            &[
                Rect::new(0.0, 0.0, 16.0, 16.0),
                Rect::new(16.0, 0.0, 16.0, 16.0),
            ],
        ));
    }
}
