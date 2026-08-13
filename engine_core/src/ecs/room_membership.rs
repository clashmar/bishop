use crate::ecs::entity::Entity;
use crate::ecs::{CurrentRoom, Ecs, TilePlacement};
use crate::tiles::TileDefId;
use crate::worlds::{RoomId, RoomLayer};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// Empty set returned when a room has no tracked entities.
static EMPTY_ROOM: Lazy<HashSet<Entity>> = Lazy::new(HashSet::new);
/// Empty set returned when a room/layer has no tracked entities.
static EMPTY_ROOM_LAYER: Lazy<HashSet<Entity>> = Lazy::new(HashSet::new);
/// Empty set returned when a tile definition has no linked placements.
static EMPTY_TILE_DEF_SET: Lazy<HashSet<Entity>> = Lazy::new(HashSet::new);
/// Empty tile map returned when a room/layer has no tracked tile entities.
static EMPTY_TILE_MAP: Lazy<HashMap<(usize, usize), Entity>> = Lazy::new(HashMap::new);

impl Ecs {
    /// Returns a reference to the set of entities currently in `room_id`.
    /// Returns an empty set if the room has no tracked entities.
    pub fn entities_in_room(&self, room_id: RoomId) -> &HashSet<Entity> {
        self.room_entities.get(&room_id).unwrap_or(&EMPTY_ROOM)
    }

    /// Returns a reference to the set of entities currently in one room/layer.
    pub fn entities_in_room_layer(&self, room_id: RoomId, layer: RoomLayer) -> &HashSet<Entity> {
        self.room_layer_entities
            .get(&(room_id, layer))
            .unwrap_or(&EMPTY_ROOM_LAYER)
    }

    /// Returns a reference to the linked tile-placement set for `tile_id`.
    pub fn tile_entities_for_definition(&self, tile_id: TileDefId) -> &HashSet<Entity> {
        self.tile_definition_entities
            .get(&tile_id)
            .unwrap_or(&EMPTY_TILE_DEF_SET)
    }

    /// Returns a reference to the room/layer/cell tile index for `room_id` + `layer`.
    pub fn tile_entities_in_room_layer(
        &self,
        room_id: RoomId,
        layer: RoomLayer,
    ) -> &HashMap<(usize, usize), Entity> {
        self.room_tile_entities
            .get(&(room_id, layer))
            .unwrap_or(&EMPTY_TILE_MAP)
    }

    /// Returns the tile placement entity occupying one room/layer/cell, if any.
    pub fn tile_entity_at(
        &self,
        room_id: RoomId,
        layer: RoomLayer,
        grid_x: usize,
        grid_y: usize,
    ) -> Option<Entity> {
        self.tile_entities_in_room_layer(room_id, layer)
            .get(&(grid_x, grid_y))
            .copied()
    }

    /// Returns the tile placement at one room/layer/cell, if any.
    pub fn tile_placement_at(
        &self,
        room_id: RoomId,
        layer: RoomLayer,
        grid_x: usize,
        grid_y: usize,
    ) -> Option<TilePlacement> {
        let entity = self.tile_entity_at(room_id, layer, grid_x, grid_y)?;
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
            self.unindex_tile_placement(current_room.room_id, current_room.layer, entity, placement);
        }

        self.unindex_room_layer_entity(current_room.room_id, current_room.layer, entity);

        if let Some(entities) = self.room_entities.get_mut(&current_room.room_id) {
            entities.remove(&entity);
            if entities.is_empty() {
                self.room_entities.remove(&current_room.room_id);
            }
        }
    }

    /// Set the `CurrentRoom` component on an entity to `new_room` on the front layer.
    ///
    /// If the entity was previously in another room it is moved out of that
    /// room's membership set. The entity must already exist.
    pub fn set_current_room(&mut self, entity: Entity, new_room: RoomId) {
        self.set_current_room_layer(entity, new_room, RoomLayer::Front);
    }

    /// Set the `CurrentRoom` component on an entity to one room/layer pair.
    pub fn set_current_room_layer(&mut self, entity: Entity, new_room: RoomId, layer: RoomLayer) {
        if self
            .get::<CurrentRoom>(entity)
            .is_some_and(|current| current.room_id == new_room && current.layer == layer)
        {
            return;
        }

        self.clear_current_room(entity);
        self.insert_component(entity, CurrentRoom::new(new_room, layer));
    }

    /// Update only the authored layer for an entity already assigned to a room.
    pub fn set_entity_layer(&mut self, entity: Entity, layer: RoomLayer) {
        let Some(current_room) = self.get::<CurrentRoom>(entity).copied() else {
            return;
        };
        self.set_current_room_layer(entity, current_room.room_id, layer);
    }

    /// Checks that an indexed room membership maps to a matching `CurrentRoom`.
    pub fn assert_room_membership(&self, room_id: RoomId, entity: Entity) {
        debug_assert!(
            self.get::<CurrentRoom>(entity)
                .is_some_and(|current_room| current_room.room_id == room_id),
            "room_entities contained {entity:?} for {room_id:?} without matching CurrentRoom"
        );
    }

    /// Rebuild `room_entities` from scratch by scanning all `CurrentRoom` components.
    pub fn rebuild_room_entities(&mut self) {
        self.room_entities.clear();
        let pairs: Vec<(Entity, RoomId)> = {
            let room_store = self.get_store::<CurrentRoom>();
            room_store
                .data
                .iter()
                .map(|(entity, current_room)| (*entity, current_room.room_id))
                .collect()
        };
        for (entity, room_id) in pairs {
            self.room_entities.entry(room_id).or_default().insert(entity);
        }
    }

    /// Rebuild `room_layer_entities` from scratch by scanning all `CurrentRoom` components.
    pub fn rebuild_room_layer_entities(&mut self) {
        self.room_layer_entities.clear();
        let pairs: Vec<(Entity, RoomId, RoomLayer)> = {
            let room_store = self.get_store::<CurrentRoom>();
            room_store
                .data
                .iter()
                .map(|(entity, current_room)| (*entity, current_room.room_id, current_room.layer))
                .collect()
        };
        for (entity, room_id, layer) in pairs {
            self.index_room_layer_entity(room_id, layer, entity);
        }
    }

    /// Rebuild `room_tile_entities` from scratch by scanning `TilePlacement`
    /// plus `CurrentRoom` components.
    pub fn rebuild_room_tile_entities(&mut self) {
        self.room_tile_entities.clear();
        let placements: Vec<(Entity, TilePlacement)> = {
            let tile_store = self.get_store::<TilePlacement>();
            tile_store
                .data
                .iter()
                .map(|(&entity, placement)| (entity, *placement))
                .collect()
        };

        for (entity, placement) in placements {
            if let Some(current_room) = self.get::<CurrentRoom>(entity).copied() {
                self.index_tile_placement(current_room.room_id, current_room.layer, entity, placement);
            }
        }
    }

    /// Rebuild `tile_definition_entities` from scratch by scanning `TilePlacement` components.
    pub fn rebuild_tile_definition_entities(&mut self) {
        self.tile_definition_entities.clear();
        let placements: Vec<(Entity, TilePlacement)> = {
            let tile_store = self.get_store::<TilePlacement>();
            tile_store
                .data
                .iter()
                .map(|(&entity, placement)| (entity, *placement))
                .collect()
        };

        for (entity, placement) in placements {
            self.index_tile_definition_entity(placement.definition, entity);
        }
    }

    pub(crate) fn index_room_layer_entity(
        &mut self,
        room_id: RoomId,
        layer: RoomLayer,
        entity: Entity,
    ) {
        self.room_layer_entities
            .entry((room_id, layer))
            .or_default()
            .insert(entity);
    }

    pub(crate) fn unindex_room_layer_entity(
        &mut self,
        room_id: RoomId,
        layer: RoomLayer,
        entity: Entity,
    ) {
        if let Some(entities) = self.room_layer_entities.get_mut(&(room_id, layer)) {
            entities.remove(&entity);
            if entities.is_empty() {
                self.room_layer_entities.remove(&(room_id, layer));
            }
        }
    }

    pub(crate) fn index_tile_definition_entity(&mut self, tile_id: TileDefId, entity: Entity) {
        self.tile_definition_entities.entry(tile_id).or_default().insert(entity);
    }

    pub(crate) fn index_tile_placement(
        &mut self,
        room_id: RoomId,
        layer: RoomLayer,
        entity: Entity,
        placement: TilePlacement,
    ) {
        let cell = (placement.grid_x, placement.grid_y);
        let previous = self
            .room_tile_entities
            .entry((room_id, layer))
            .or_default()
            .insert(cell, entity);
        debug_assert!(
            previous.is_none() || previous == Some(entity),
            "room_tile_entities replaced {:?} at {:?} in {:?}/{:?}",
            previous,
            cell,
            room_id,
            layer,
        );
    }

    pub(crate) fn unindex_tile_placement(
        &mut self,
        room_id: RoomId,
        layer: RoomLayer,
        entity: Entity,
        placement: TilePlacement,
    ) {
        let cell = (placement.grid_x, placement.grid_y);
        if let Some(tiles) = self.room_tile_entities.get_mut(&(room_id, layer)) {
            if tiles.get(&cell).copied() == Some(entity) {
                tiles.remove(&cell);
            }
            if tiles.is_empty() {
                self.room_tile_entities.remove(&(room_id, layer));
            }
        }
    }

    pub(crate) fn unindex_tile_definition_entity(&mut self, tile_id: TileDefId, entity: Entity) {
        if let Some(entities) = self.tile_definition_entities.get_mut(&tile_id) {
            entities.remove(&entity);
            if entities.is_empty() {
                self.tile_definition_entities.remove(&tile_id);
            }
        }
    }
}
