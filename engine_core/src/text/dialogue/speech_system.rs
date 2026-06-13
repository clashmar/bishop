use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::speech_bubble::SpeechBubble;
use crate::worlds::world::{entity_in_world, World};

/// Updates speech bubble timers for `world`'s entities and removes expired bubbles.
pub fn update_speech_timers(ecs: &mut Ecs, world: &World, dt: f32) {
    let eligible: Vec<Entity> = ecs
        .get_store::<SpeechBubble>()
        .data
        .keys()
        .filter(|entity| entity_in_world(ecs, world, **entity))
        .copied()
        .collect();

    let store = ecs.get_store_mut::<SpeechBubble>();
    for entity in eligible {
        let Some(bubble) = store.data.get_mut(&entity) else {
            continue;
        };
        bubble.timer -= dt;
        if bubble.timer <= 0.0 {
            store.remove(entity);
        }
    }
}

/// Removes the speech bubble from an entity immediately.
pub fn clear_speech(ecs: &mut Ecs, entity: Entity) {
    ecs.get_store_mut::<SpeechBubble>().remove(entity);
}

/// Checks if an entity currently has a speech bubble.
pub fn is_speaking(ecs: &Ecs, entity: Entity) -> bool {
    ecs.get_store::<SpeechBubble>().contains(entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::test_utils::make_room;
    use crate::worlds::world::{World, WorldId};

    fn bubble(timer: f32) -> SpeechBubble {
        SpeechBubble {
            timer,
            ..Default::default()
        }
    }

    fn world_with_room_one() -> World {
        World::from_rooms(
            WorldId(1),
            String::new(),
            vec![make_room(Some(1), 0.0, 0.0, 4.0, 4.0)],
            16.0,
        )
    }

    #[test]
    fn speech_timers_tick_for_active_world_entities() {
        let mut ecs = Ecs::default();
        let world = world_with_room_one();
        let speaker = ecs
            .create_entity()
            .with(bubble(1.0))
            .with_current_room(crate::worlds::room::RoomId(1))
            .finish();

        update_speech_timers(&mut ecs, &world, 0.25);

        assert_eq!(ecs.get::<SpeechBubble>(speaker).map(|b| b.timer), Some(0.75));
    }

    #[test]
    fn speech_timers_freeze_for_inactive_world_entities() {
        let mut ecs = Ecs::default();
        let world = world_with_room_one();
        let parked = ecs
            .create_entity()
            .with(bubble(1.0))
            .with_current_room(crate::worlds::room::RoomId(99))
            .finish();

        update_speech_timers(&mut ecs, &world, 0.25);

        assert_eq!(ecs.get::<SpeechBubble>(parked).map(|b| b.timer), Some(1.0));
    }

    #[test]
    fn speech_timers_remove_expired_bubbles() {
        let mut ecs = Ecs::default();
        let world = world_with_room_one();
        let speaker = ecs.create_entity().with(bubble(0.1)).finish();

        update_speech_timers(&mut ecs, &world, 0.25);

        assert!(!is_speaking(&ecs, speaker));
    }
}
