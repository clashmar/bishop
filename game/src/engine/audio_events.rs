use super::Engine;
use engine_core::audio::runtime;
use engine_core::{onscreen_error, onscreen_log};
use mlua::{Value, Variadic};

pub(super) fn emit_pending_audio_events(engine: &Engine) {
    let events = runtime::drain_audio_events();
    if events.is_empty() {
        return;
    }

    let event_bus = engine
        .game_instance
        .borrow()
        .game
        .script_manager
        .event_bus
        .clone();
    for event in events {
        let Ok(payload) = engine.lua.create_table() else {
            onscreen_error!("Failed to create audio event payload table");
            continue;
        };

        if payload.set("id", event.id).is_err()
            || payload.set("reason", event.reason.as_str()).is_err()
            || payload.set("next_id", event.next_id).is_err()
        {
            onscreen_error!("Failed to populate audio event payload table");
            continue;
        }

        event_bus.emit(
            "audio:music_stopped".to_string(),
            Variadic::from_iter([Value::Table(payload)]),
        );
    }
}
