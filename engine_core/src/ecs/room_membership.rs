use crate::ecs::CurrentRoom;
use crate::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::worlds::room::RoomId;
use once_cell::sync::Lazy;
use std::collections::HashSet;

/// Empty set returned when a room has no tracked entities.
static EMPTY_ROOM: Lazy<HashSet<Entity>> = Lazy::new(HashSet::new);

impl Ecs {
    /// Returns a reference to the set of entities currently in `room_id`.
    /// Returns an empty set if the room has no tracked entities.
    pub fn entities_in_room(&self, room_id: RoomId) -> &HashSet<Entity> {
        self.room_entities.get(&room_id).unwrap_or(&EMPTY_ROOM)
    }

    /// Set the `CurrentRoom` component on an entity to `new_room`.
    ///
    /// If the entity was previously in another room it is moved out of that
    /// room's membership set.  The entity must already exist.  If the entity
    /// does not currently have a `CurrentRoom` component, one is inserted.
    pub fn set_current_room(&mut self, entity: Entity, new_room: RoomId) {
        // Remove from old room membership directly (avoid scanning all rooms)
        let old_room = self.get::<CurrentRoom>(entity).map(|cr| cr.0);
        if let Some(old_room) = old_room {
            if let Some(entities) = self.room_entities.get_mut(&old_room) {
                entities.remove(&entity);
                if entities.is_empty() {
                    self.room_entities.remove(&old_room);
                }
            }
        }

        // Insert (or overwrite) the CurrentRoom component.
        // on_insert on CurrentRoom will add the entity to `room_entities`.
        self.insert_component(entity, CurrentRoom(new_room));
    }

    /// Rebuild `room_entities` from scratch by scanning all `CurrentRoom`
    /// components.  Called by `finalize_after_load`.
    pub(crate) fn rebuild_room_entities(&mut self) {
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
}
