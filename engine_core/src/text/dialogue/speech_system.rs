// engine_core/src/text/dialogue/speech_system.rs
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::text::dialogue::SpeechBubble;

/// Updates speech bubble timers and removes expired bubbles.
pub fn update_speech_timers(ecs: &mut Ecs, dt: f32) {
    let store = ecs.get_store_mut::<SpeechBubble>();

    let expired: Vec<Entity> = store
        .data
        .iter_mut()
        .filter_map(|(entity, bubble)| {
            bubble.timer -= dt;
            if bubble.timer <= 0.0 {
                Some(*entity)
            } else {
                None
            }
        })
        .collect();

    for entity in expired {
        store.remove(entity);
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
