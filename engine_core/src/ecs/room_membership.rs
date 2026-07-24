use crate::ecs::{CurrentRoom, Ecs, TilePlacement};
use crate::ecs::entity::Entity;
use crate::worlds::room::RoomId;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// Empty set returned when a room has no tracked entities.
static EMPTY_ROOM: Lazy<HashSet<Entity>> = Lazy::new(HashSet::new);
/// Empty tile map returned when a room has no tracked tile entities.
static EMPTY_TILE_MAP: Lazy<HashMap<(usize, usize), Entity>> = Lazy::new(HashMap::new);

impl Ecs {
    /// Returns a reference to the set of entities currently in `room_id`.
    /// Returns an empty set if the room has no tracked entities.
    pub fn entities_in_room(&self, room_id: RoomId) -> &HashSet<Entity> {
        self.room_entities.get(&room_id).unwrap_or(&EMPTY_ROOM)
    }

    /// Returns a reference to the room/cell tile index for `room_id`.
    pub fn tile_entities_in_room(&self, room_id: RoomId) -> &HashMap<(usize, usize), Entity> {
        self.room_tile_entities.get(&room_id).unwrap_or(&EMPTY_TILE_MAP)
    }

    /// Returns the tile placement entity occupying one room/cell, if any.
    pub fn tile_entity_at(&self, room_id: RoomId, grid_x: usize, grid_y: usize) -> Option<Entity> {
        self.tile_entities_in_room(room_id)
            .get(&(grid_x, grid_y))
            .copied()
    }

    /// Returns the tile placement at one room/cell, if any.
    pub fn tile_placement_at(
        &self,
        room_id: RoomId,
        grid_x: usize,
        grid_y: usize,
    ) -> Option<TilePlacement> {
        let entity = self.tile_entity_at(room_id, grid_x, grid_y)?;
        self.get::<TilePlacement>(entity).copied()
    }

    /// Removes room membership for an entity if it has one.
    pub fn clear_current_room(&mut self, entity: Entity) {
        let Some(current_room) = self.get::<CurrentRoom>(entity).copied() else {
            return;
        };
        let tile_placement = self.get::<TilePlacement>(entity).copied();

        self.get_store_mut::<CurrentRoom>().remove(entity);

        if let Some(placement) = tile_placement {
            self.unindex_tile_placement(current_room.0, entity, placement);
        }

        if let Some(entities) = self.room_entities.get_mut(&current_room.0) {
            entities.remove(&entity);
            if entities.is_empty() {
                self.room_entities.remove(&current_room.0);
            }
        }
    }

    /// Set the `CurrentRoom` component on an entity to `new_room`.
    ///
    /// If the entity was previously in another room it is moved out of that
    /// room's membership set. The entity must already exist.
    pub fn set_current_room(&mut self, entity: Entity, new_room: RoomId) {
        if self
            .get::<CurrentRoom>(entity)
            .is_some_and(|current| current.0 == new_room)
        {
            return;
        }

        self.clear_current_room(entity);
        self.insert_component(entity, CurrentRoom(new_room));
    }

    /// Checks that an indexed room membership maps to a matching `CurrentRoom`.
    pub fn assert_room_membership(&self, room_id: RoomId, entity: Entity) {
        debug_assert!(
            self.get::<CurrentRoom>(entity)
                .is_some_and(|current_room| current_room.0 == room_id),
            "room_entities contained {entity:?} for {room_id:?} without matching CurrentRoom"
        );
    }

    /// Rebuild `room_entities` from scratch by scanning all `CurrentRoom` components.
    pub fn rebuild_room_entities(&mut self) {
        self.room_entities.clear();
        // Collect to avoid borrow conflicts with the store
        let pairs: Vec<(Entity, RoomId)> = {
            let room_store = self.get_store::<CurrentRoom>();
            room_store.data.iter().map(|(e, cr)| (*e, cr.0)).collect()
        };
        for (entity, room_id) in pairs {
            self.room_entities
                .entry(room_id)
                .or_default()
                .insert(entity);
        }
    }

    /// Rebuild `room_tile_entities` from scratch by scanning `TilePlacement`
    /// plus `CurrentRoom` components.
    pub fn rebuild_room_tile_entities(&mut self) {
        self.room_tile_entities.clear();
        let placements: Vec<(Entity, TilePlacement)> = {
            let tile_store = self.get_store::<TilePlacement>();
            tile_store.data.iter().map(|(&entity, placement)| (entity, *placement)).collect()
        };

        for (entity, placement) in placements {
            if let Some(room_id) = self.get::<CurrentRoom>(entity).map(|room| room.0) {
                self.index_tile_placement(room_id, entity, placement);
            }
        }
    }

    pub(crate) fn index_tile_placement(
        &mut self,
        room_id: RoomId,
        entity: Entity,
        placement: TilePlacement,
    ) {
        let cell = (placement.grid_x, placement.grid_y);
        let previous = self
            .room_tile_entities
            .entry(room_id)
            .or_default()
            .insert(cell, entity);
        debug_assert!(
            previous.is_none() || previous == Some(entity),
            "room_tile_entities replaced {:?} at {:?} in {:?}",
            previous,
            cell,
            room_id,
        );
    }

    pub(crate) fn unindex_tile_placement(
        &mut self,
        room_id: RoomId,
        entity: Entity,
        placement: TilePlacement,
    ) {
        let cell = (placement.grid_x, placement.grid_y);
        if let Some(tiles) = self.room_tile_entities.get_mut(&room_id) {
            if tiles.get(&cell).copied() == Some(entity) {
                tiles.remove(&cell);
            }
            if tiles.is_empty() {
                self.room_tile_entities.remove(&room_id);
            }
        }
    }
}
