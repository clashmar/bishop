use super::Room;
use crate::ecs::{Ecs, Entity};
use crate::worlds::RoomLayer;
use bishop::prelude::Vec2;
use std::collections::{HashMap, VecDeque};

/// Integer grid cell coordinates in world grid space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridCell {
    pub x: i32,
    pub y: i32,
}

/// Result status for grid navigation queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridQueryStatus {
    Ok,
    Blocked,
    NoPath,
    BudgetExhausted,
    InvalidEndpoint,
}

/// Bounded path query result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridPathResult {
    pub status: GridQueryStatus,
    pub path: Vec<GridCell>,
}

/// Returns the grid cell containing a world-space position.
pub fn cell_at(position: Vec2, grid_size: f32) -> GridCell {
    GridCell {
        x: (position.x / grid_size).floor() as i32,
        y: (position.y / grid_size).floor() as i32,
    }
}

/// Returns the world-space center of a grid cell.
pub fn cell_center(cell: GridCell, grid_size: f32) -> Vec2 {
    Vec2::new(
        (cell.x as f32 + 0.5) * grid_size,
        (cell.y as f32 + 0.5) * grid_size,
    )
}

/// Returns the tile entity blocking a cell inside the room's bounds.
pub fn blocked_tile_entity(
    ecs: &Ecs,
    room: &Room,
    grid_size: f32,
    layer: RoomLayer,
    cell: GridCell,
) -> Option<Entity> {
    let bounds = SearchBounds::for_room(room, grid_size)?;
    let (local_x, local_y) = bounds.local_cell(cell)?;
    ecs.tile_entity_at(room.id, layer, local_x, local_y)
}

/// Returns whether an actor can step onto a cell within the room's bounds.
pub fn can_step(
    ecs: &Ecs,
    room: &Room,
    grid_size: f32,
    layer: RoomLayer,
    target: GridCell,
) -> bool {
    let Some(bounds) = SearchBounds::for_room(room, grid_size) else {
        return false;
    };
    let Some((local_x, local_y)) = bounds.local_cell(target) else {
        return false;
    };

    ecs.tile_entity_at(room.id, layer, local_x, local_y)
        .is_none()
}

/// Finds a bounded cardinal path between two cells within the room's bounds.
pub fn find_path(
    ecs: &Ecs,
    room: &Room,
    grid_size: f32,
    layer: RoomLayer,
    start: GridCell,
    target: GridCell,
    max_steps: usize,
) -> GridPathResult {
    if max_steps == 0 {
        return status(GridQueryStatus::BudgetExhausted);
    }
    let Some(bounds) = SearchBounds::for_room(room, grid_size) else {
        return status(GridQueryStatus::InvalidEndpoint);
    };
    if !bounds.contains(start)
        || !bounds.contains(target)
        || !can_step(ecs, room, grid_size, layer, start)
        || !can_step(ecs, room, grid_size, layer, target)
    {
        return status(GridQueryStatus::InvalidEndpoint);
    }
    if start == target {
        return GridPathResult {
            status: GridQueryStatus::Ok,
            path: vec![start],
        };
    }

    let mut frontier = VecDeque::from([start]);
    let mut came_from = HashMap::from([(start, None)]);
    let mut expanded = 0;

    while let Some(current) = frontier.pop_front() {
        if expanded >= max_steps {
            return status(GridQueryStatus::BudgetExhausted);
        }
        expanded += 1;

        for next in cardinal_neighbors(current) {
            if !bounds.contains(next)
                || came_from.contains_key(&next)
                || !can_step(ecs, room, grid_size, layer, next)
            {
                continue;
            }

            came_from.insert(next, Some(current));
            if next == target {
                return GridPathResult {
                    status: GridQueryStatus::Ok,
                    path: reconstruct_path(start, target, &came_from),
                };
            }
            frontier.push_back(next);
        }
    }

    status(GridQueryStatus::NoPath)
}

fn status(status: GridQueryStatus) -> GridPathResult {
    GridPathResult {
        status,
        path: Vec::new(),
    }
}

fn cardinal_neighbors(cell: GridCell) -> [GridCell; 4] {
    [
        GridCell {
            x: cell.x + 1,
            y: cell.y,
        },
        GridCell {
            x: cell.x - 1,
            y: cell.y,
        },
        GridCell {
            x: cell.x,
            y: cell.y + 1,
        },
        GridCell {
            x: cell.x,
            y: cell.y - 1,
        },
    ]
}

fn reconstruct_path(
    start: GridCell,
    target: GridCell,
    came_from: &HashMap<GridCell, Option<GridCell>>,
) -> Vec<GridCell> {
    let mut path = vec![target];
    let mut current = target;

    while current != start {
        let Some(Some(previous)) = came_from.get(&current).copied() else {
            break;
        };
        path.push(previous);
        current = previous;
    }

    path.reverse();
    path
}

struct SearchBounds {
    origin: GridCell,
    width: i32,
    height: i32,
}

impl SearchBounds {
    fn for_room(room: &Room, grid_size: f32) -> Option<Self> {
        if !grid_size.is_finite() || grid_size <= 0.0 {
            return None;
        }

        let width = room.size.x.ceil() as i32;
        let height = room.size.y.ceil() as i32;
        if width <= 0 || height <= 0 {
            return None;
        }

        Some(Self {
            origin: cell_at(room.position, grid_size),
            width,
            height,
        })
    }

    fn contains(&self, cell: GridCell) -> bool {
        let Some(local_x) = cell.x.checked_sub(self.origin.x) else {
            return false;
        };
        let Some(local_y) = cell.y.checked_sub(self.origin.y) else {
            return false;
        };
        local_x >= 0 && local_x < self.width && local_y >= 0 && local_y < self.height
    }

    fn local_cell(&self, cell: GridCell) -> Option<(usize, usize)> {
        if !self.contains(cell) {
            return None;
        }
        let x = usize::try_from(cell.x.checked_sub(self.origin.x)?).ok()?;
        let y = usize::try_from(cell.y.checked_sub(self.origin.y)?).ok()?;
        Some((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{CurrentRoom, TilePlacement};
    use crate::tiles::TileDefId;
    use crate::worlds::RoomId;

    fn test_room(id: usize, position: Vec2, size: Vec2) -> Room {
        Room {
            id: RoomId(id),
            position,
            size,
            ..Default::default()
        }
    }

    fn place_tile(ecs: &mut Ecs, room_id: RoomId, cell: GridCell) {
        let entity = ecs.create_entity().finish();
        ecs.insert_component(
            entity,
            TilePlacement::new(TileDefId(3), cell.x as usize, cell.y as usize),
        );
        ecs.insert_component(entity, CurrentRoom::new(room_id, RoomLayer::Front));
    }

    #[test]
    fn cell_at_uses_world_grid_size() {
        assert_eq!(cell_at(Vec2::new(17.0, 31.0), 16.0), GridCell { x: 1, y: 1 });
    }

    #[test]
    fn blocked_tile_entity_translates_world_cell_to_room_local_cell() {
        let mut ecs = Ecs::default();
        let room = test_room(5, Vec2::new(32.0, 16.0), Vec2::new(4.0, 4.0));
        let local_cell = GridCell { x: 2, y: 1 };
        let world_cell = GridCell { x: 4, y: 2 };
        place_tile(&mut ecs, room.id, local_cell);

        assert!(blocked_tile_entity(&ecs, &room, 16.0, RoomLayer::Front, world_cell).is_some());
    }

    #[test]
    fn can_step_when_target_has_tile_returns_false() {
        let mut ecs = Ecs::default();
        let room = test_room(5, Vec2::ZERO, Vec2::new(8.0, 8.0));
        let cell = GridCell { x: 2, y: 4 };
        place_tile(&mut ecs, room.id, cell);

        assert!(!can_step(&ecs, &room, 16.0, RoomLayer::Front, cell));
    }

    #[test]
    fn can_step_when_target_is_outside_room_returns_false() {
        let ecs = Ecs::default();
        let room = test_room(5, Vec2::new(32.0, 16.0), Vec2::new(2.0, 3.0));

        assert!(!can_step(
            &ecs,
            &room,
            16.0,
            RoomLayer::Front,
            GridCell { x: 1, y: 1 },
        ));
        assert!(!can_step(
            &ecs,
            &room,
            16.0,
            RoomLayer::Front,
            GridCell { x: 4, y: 3 },
        ));
    }

    #[test]
    fn find_path_does_not_route_outside_room_around_blocked_row() {
        let mut ecs = Ecs::default();
        let room = test_room(5, Vec2::ZERO, Vec2::new(3.0, 1.0));
        place_tile(&mut ecs, room.id, GridCell { x: 1, y: 0 });

        let result = find_path(
            &ecs,
            &room,
            16.0,
            RoomLayer::Front,
            GridCell { x: 0, y: 0 },
            GridCell { x: 2, y: 0 },
            20,
        );

        assert_eq!(result.status, GridQueryStatus::NoPath);
        assert!(result.path.is_empty());
    }

    #[test]
    fn find_path_rejects_endpoints_outside_room_bounds() {
        let ecs = Ecs::default();
        let room = test_room(5, Vec2::ZERO, Vec2::new(3.0, 2.0));

        let result = find_path(
            &ecs,
            &room,
            16.0,
            RoomLayer::Front,
            GridCell { x: 0, y: 0 },
            GridCell { x: 3, y: 0 },
            20,
        );

        assert_eq!(result.status, GridQueryStatus::InvalidEndpoint);
        assert!(result.path.is_empty());
    }

    #[test]
    fn find_path_when_budget_exhausted_returns_budget_status() {
        let ecs = Ecs::default();
        let room = test_room(5, Vec2::ZERO, Vec2::new(16.0, 16.0));

        let result = find_path(
            &ecs,
            &room,
            16.0,
            RoomLayer::Front,
            GridCell { x: 0, y: 0 },
            GridCell { x: 10, y: 0 },
            2,
        );

        assert_eq!(result.status, GridQueryStatus::BudgetExhausted);
        assert!(result.path.is_empty());
    }
}
