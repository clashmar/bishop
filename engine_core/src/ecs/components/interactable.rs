use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::{CurrentRoom, LayerDoor, Transform};
use crate::worlds::{RoomId, RoomLayer};
use bishop::prelude::{Rect, Vec2};
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Semantic interactable area kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractableShape {
    #[default]
    Circle,
    Rect,
}

impl InteractableShape {
    pub const ALL: [Self; 2] = [Self::Circle, Self::Rect];

    pub fn ui_label(self) -> &'static str {
        match self {
            InteractableShape::Circle => "Circle",
            InteractableShape::Rect => "Rect",
        }
    }
}

impl std::fmt::Display for InteractableShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.ui_label())
    }
}

/// Component for interactable entities.
#[ecs_component]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct Interactable {
    /// When true, the authored interactable area is rectangular instead of circular.
    pub use_rect: bool,
    /// Local offset from the entity origin to the interactable area's center.
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub offset: Vec2,
    /// Radius used when `shape == InteractableShape::Circle`.
    pub radius: f32,
    /// Rect size used when `shape == InteractableShape::Rect`.
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub rect_size: Vec2,
    // TODO: Add priority,
    // enabled/disabled,
    // prompt,
    // facing
    // event dispatch
}

impl Default for Interactable {
    fn default() -> Self {
        Self {
            use_rect: false,
            offset: Vec2::ZERO,
            radius: 20.0,
            rect_size: Vec2::splat(16.0),
        }
    }
}

impl Interactable {
    /// Creates a circular interactable area.
    pub fn circle(offset: Vec2, radius: f32) -> Self {
        Self {
            use_rect: false,
            offset,
            radius,
            ..Default::default()
        }
    }

    /// Creates a rectangular interactable area.
    pub fn rect(offset: Vec2, rect_size: Vec2) -> Self {
        Self {
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

    /// Returns the center of the authored interactable area in world space.
    pub fn center_at(&self, origin: Vec2) -> Vec2 {
        origin + self.offset
    }

    /// Returns the world-space bounds enclosing this interactable area.
    pub fn bounds_at(&self, origin: Vec2) -> Rect {
        let center = self.center_at(origin);
        match self.shape() {
            InteractableShape::Circle => Rect::new(
                center.x - self.radius,
                center.y - self.radius,
                self.radius * 2.0,
                self.radius * 2.0,
            ),
            InteractableShape::Rect => Rect::new(
                center.x - self.rect_size.x * 0.5,
                center.y - self.rect_size.y * 0.5,
                self.rect_size.x,
                self.rect_size.y,
            ),
        }
    }

    /// Returns true when a world-space point is inside the authored interactable area.
    pub fn contains_point(&self, origin: Vec2, point: Vec2) -> bool {
        match self.shape() {
            InteractableShape::Circle => self.center_at(origin).distance(point) <= self.radius,
            InteractableShape::Rect => self.bounds_at(origin).contains(point),
        }
    }

    /// Returns the selection distance used to pick the closest interactable.
    pub fn selection_distance(&self, origin: Vec2, point: Vec2) -> f32 {
        self.center_at(origin).distance(point)
    }

    /// Returns true when this interactable area is fully contained in the supplied rect union.
    pub fn area_contained_in_bounds(&self, origin: Vec2, bounds: &[Rect]) -> bool {
        let area_bounds = self.bounds_at(origin);
        match self.shape() {
            InteractableShape::Rect => area_bounds.contained_in_union(bounds),
            InteractableShape::Circle => {
                let center = self.center_at(origin);
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

/// Returns the closest interactable entity candidate reachable from one room/layer pair.
pub fn find_best_interactable_in_layer(
    ecs: &Ecs,
    room_id: RoomId,
    layer: RoomLayer,
    source_pos: Vec2,
) -> Option<Entity> {
    let mut best: Option<(Entity, f32)> = None;
    consider_interactable_candidates(ecs, room_id, layer, source_pos, false, &mut best);
    if layer == RoomLayer::Back {
        consider_interactable_candidates(ecs, room_id, RoomLayer::Front, source_pos, true, &mut best);
    }
    best.map(|(entity, _)| entity)
}

fn consider_interactable_candidates(
    ecs: &Ecs,
    room_id: RoomId,
    layer: RoomLayer,
    source_pos: Vec2,
    layer_doors_only: bool,
    best: &mut Option<(Entity, f32)>,
) {
    let interactables = ecs.get_store::<Interactable>();
    let positions = ecs.get_store::<Transform>();

    for &entity in ecs.entities_in_room_layer(room_id, layer) {
        ecs.assert_room_membership(room_id, entity);

        if layer_doors_only && !ecs.has::<LayerDoor>(entity) {
            continue;
        }

        let Some(interactable) = interactables.get(entity) else {
            continue;
        };
        let Some(position) = positions.get(entity).map(|transform| transform.position) else {
            continue;
        };
        if !interactable.contains_point(position, source_pos) {
            continue;
        }

        let distance = interactable.selection_distance(position, source_pos);
        match best {
            None => *best = Some((entity, distance)),
            Some((_, best_distance)) if distance < *best_distance => {
                *best = Some((entity, distance));
            }
            _ => {}
        }
    }
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
            .with(Interactable::circle(Vec2::ZERO, 100.0))
            .with_current_room(room_id)
            .finish();

        let player = ecs.create_entity()
            .with(Transform {
                position: Vec2::new(0.0, 0.0),
                ..Default::default()
            })
            .with(crate::ecs::Player)
            .with_current_room(room_id)
            .finish();

        ecs.create_entity()
            .with(Transform {
                position: Vec2::new(1.0, 0.0),
                ..Default::default()
            })
            .with(Interactable::circle(Vec2::ZERO, 100.0))
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
            .with(crate::ecs::Player)
            .with_current_room(player_room)
            .finish();

        ecs.create_entity()
            .with(Transform {
                position: Vec2::new(1.0, 0.0),
                ..Default::default()
            })
            .with(Interactable::circle(Vec2::ZERO, 100.0))
            .with_current_room(other_room)
            .finish();

        assert_eq!(ecs.get_player_entity(), Some(player));
        assert_eq!(find_best_interactable(&ecs), None);
    }

    #[test]
    fn find_best_interactable_uses_rect_area_instead_of_entity_distance() {
        let mut ecs = Ecs::default();
        let room_id = RoomId(3);

        let rect_entity = ecs.create_entity()
            .with(Transform::default())
            .with(Interactable::rect(Vec2::new(32.0, 0.0), Vec2::new(16.0, 16.0)))
            .with_current_room(room_id)
            .finish();

        let player = ecs.create_entity()
            .with(Transform {
                position: Vec2::new(32.0, 0.0),
                ..Default::default()
            })
            .with(crate::ecs::Player)
            .with_current_room(room_id)
            .finish();

        assert_eq!(ecs.get_player_entity(), Some(player));
        assert_eq!(find_best_interactable(&ecs), Some(rect_entity));
    }

    #[test]
    fn find_best_interactable_on_back_layer_can_use_front_layer_door() {
        let mut ecs = Ecs::default();
        let room_id = RoomId(3);

        let door_entity = ecs.create_entity()
            .with(Transform::default())
            .with(Interactable::circle(Vec2::ZERO, 12.0))
            .with(crate::ecs::LayerDoor::default())
            .with_current_room(room_id)
            .finish();

        let player = ecs.create_entity()
            .with(Transform {
                position: Vec2::new(0.0, 0.0),
                ..Default::default()
            })
            .with(crate::ecs::Player)
            .with_current_room_layer(room_id, RoomLayer::Back)
            .finish();

        assert_eq!(ecs.get_player_entity(), Some(player));
        assert_eq!(find_best_interactable(&ecs), Some(door_entity));
    }

    #[test]
    fn interactable_area_contained_in_zone_union_when_rect_then_validation_passes() {
        let interactable = Interactable::rect(Vec2::ZERO, Vec2::new(16.0, 16.0));

        assert!(interactable.area_contained_in_bounds(
            Vec2::new(8.0, 8.0),
            &[
                Rect::new(0.0, 0.0, 16.0, 16.0),
                Rect::new(16.0, 0.0, 16.0, 16.0),
            ],
        ));
    }
}
