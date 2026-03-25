// engine_core/src/scripting/event_bus.rs
use crate::ecs::entity::Entity;
use crate::*;
use std::collections::HashMap;
use std::sync::Mutex;
use mlua::Variadic;
use mlua::UserData;
use std::sync::Arc;
use mlua::Function;
use mlua::Value;

/// A listener with an optional entity association for cleanup.
#[derive(Clone)]
struct Listener {
    /// The entity that owns this listener (None for global listeners).
    entity: Option<Entity>,
    /// The Lua callback function.
    handler: Function,
}

/// Event dispatcher that stores Lua callbacks.
#[derive(Default, Clone)]
pub struct EventBus {
    /// Maps an event name to list of listeners.
    listeners: Arc<Mutex<HashMap<String, Vec<Listener>>>>,
}

impl UserData for EventBus {}

impl EventBus {
    /// Register a listener for `event`. The `Function` is a Lua closure.
    pub fn on(&self, event: String, handler: Function) {
        let mut map = self.listeners.lock().unwrap();
        map.entry(event).or_default().push(Listener {
            entity: None,
            handler,
        });
    }

    /// Register a listener for `event` associated with a specific entity.
    pub fn on_for_entity(&self, event: String, entity: Entity, handler: Function) {
        let mut map = self.listeners.lock().unwrap();
        map.entry(event).or_default().push(Listener {
            entity: Some(entity),
            handler,
        });
    }

    /// Emit an event. All registered handlers are called with the supplied arguments.
    pub fn emit(&self, event: String, args: Variadic<Value>) {
        let map = self.listeners.lock().unwrap();
        if let Some(listeners) = map.get(&event) {
            for listener in listeners {
                if let Err(e) = listener.handler.call::<()>(args.clone()) {
                    onscreen_error!("Lua listener error for event '{}': {}", event, e);
                }
            }
        }
    }

    /// Remove all listeners associated with the given entity.
    pub fn remove_entity_listeners(&self, entity: Entity) {
        let mut map = self.listeners.lock().unwrap();
        for listeners in map.values_mut() {
            listeners.retain(|l| l.entity != Some(entity));
        }
        // Remove empty event entries
        map.retain(|_, v| !v.is_empty());
    }

    /// Clear all listeners.
    pub fn clear(&self) {
        let mut map = self.listeners.lock().unwrap();
        map.clear();
    }

    /// Get the total number of registered listeners.
    pub fn listener_count(&self) -> usize {
        let map = self.listeners.lock().unwrap();
        map.values().map(|v| v.len()).sum()
    }

    /// Get the number of listeners for a specific event.
    pub fn listener_count_for_event(&self, event: &str) -> usize {
        let map = self.listeners.lock().unwrap();
        map.get(event).map(|v| v.len()).unwrap_or(0)
    }

    /// Get the number of listeners associated with a specific entity.
    pub fn listener_count_for_entity(&self, entity: Entity) -> usize {
        let map = self.listeners.lock().unwrap();
        map.values()
            .flat_map(|v| v.iter())
            .filter(|l| l.entity == Some(entity))
            .count()
    }
}
