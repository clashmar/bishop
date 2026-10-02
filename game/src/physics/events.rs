use crate::scripting::commands::entity::lookup_entity_function;
use crate::scripting::modules::entity_module::lua_entity_handle;
use engine_core::ecs::Entity;
use engine_core::game::Game;
use engine_core::logging::omni_error;
use engine_core::scripting::lua_constants::{
    lua_collision,
    lua_events,
    lua_kinematic,
    lua_sensor,
};
use mlua::{Lua, Table, Value, Variadic};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysicsEvent {
    KinematicContact(KinematicContactEvent),
    Sensor(SensorEvent),
    Collision(CollisionEvent),
}

#[derive(Default)]
pub(crate) struct PhysicsEvents {
    queued: Vec<PhysicsEvent>,
}

impl PhysicsEvents {
    pub(crate) fn push_kinematic_contact(&mut self, event: KinematicContactEvent) {
        self.queued.push(PhysicsEvent::KinematicContact(event));
    }

    pub(crate) fn push_sensor(&mut self, event: SensorEvent) {
        self.queued.push(PhysicsEvent::Sensor(event));
    }

    pub(crate) fn push_collision(&mut self, event: CollisionEvent) {
        self.queued.push(PhysicsEvent::Collision(event));
    }

    pub(crate) fn drain(&mut self) -> Vec<PhysicsEvent> {
        std::mem::take(&mut self.queued)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KinematicContactEvent {
    Contact { kinematic: Entity, dynamic: Entity },
}

impl KinematicContactEvent {
    /// Returns the kinematic entity involved in this contact event.
    pub(crate) fn kinematic(self) -> Entity {
        match self {
            Self::Contact { kinematic, .. } => kinematic,
        }
    }

    /// Returns the non-kinematic entity involved in this contact event.
    pub(crate) fn other(self) -> Entity {
        match self {
            Self::Contact { dynamic, .. } => dynamic,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CollisionEvent {
    Enter { first: Entity, second: Entity },
    Exit { first: Entity, second: Entity },
    Squeeze { kinematic: Entity, dynamic: Entity },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SensorEvent {
    Enter { body: Entity, sensor: Entity },
    Stay { body: Entity, sensor: Entity },
    Exit { body: Entity, sensor: Entity },
}

impl SensorEvent {
    /// Returns the moving body involved in this sensor event.
    pub(crate) fn body(self) -> Entity {
        match self {
            Self::Enter { body, .. } | Self::Stay { body, .. } | Self::Exit { body, .. } => body,
        }
    }

    /// Returns the sensor entity involved in this sensor event.
    pub(crate) fn sensor(self) -> Entity {
        match self {
            Self::Enter { sensor, .. } | Self::Stay { sensor, .. } | Self::Exit { sensor, .. } => sensor,
        }
    }
}

struct LocalPhysicsCallback<'a> {
    entity: Entity,
    other: Option<Entity>,
    role: &'static str,
    callback_name: &'static str,
    payload: &'a Table,
    event_label: &'static str,
    self_entity_key: &'static str,
    other_key: Option<&'static str>,
    role_key: &'static str,
}

/// Drains retained physics events and forwards them to Lua listeners/callbacks.
pub(crate) fn emit_physics_events(lua: &Lua, game: &Game, events: &mut PhysicsEvents) {
    for event in events.drain() {
        match event {
            PhysicsEvent::KinematicContact(contact) => emit_kinematic_global_event(lua, game, contact),
            PhysicsEvent::Sensor(sensor) => emit_sensor_global_event(lua, game, sensor),
            PhysicsEvent::Collision(collision) => emit_collision_global_event(lua, game, collision),
        }
    }
}

fn emit_kinematic_global_event(lua: &Lua, game: &Game, event: KinematicContactEvent) {
    let Ok(payload) = kinematic_event_payload(lua, event) else {
        omni_error!("Failed to build kinematic Lua event payload for {:?}", event);
        return;
    };

    game.script_manager.event_bus.emit(
        kinematic_event_name(event).to_string(),
        Variadic::from_iter([Value::Table(payload.clone())]),
    );
    emit_kinematic_local_callbacks(lua, game, event, &payload);
}

fn kinematic_event_payload(lua: &Lua, event: KinematicContactEvent) -> mlua::Result<Table> {
    let payload = lua.create_table()?;
    payload.set(
        lua_kinematic::EVENT_KINEMATIC,
        lua_entity_handle(lua, event.kinematic())?,
    )?;
    payload.set(
        lua_kinematic::EVENT_OTHER,
        lua_entity_handle(lua, event.other())?,
    )?;
    payload.set(lua_kinematic::EVENT_KIND, kinematic_event_kind(event))?;
    Ok(payload)
}

fn emit_kinematic_local_callbacks(
    lua: &Lua,
    game: &Game,
    event: KinematicContactEvent,
    payload: &Table,
) {
    let callback_name = match event {
        KinematicContactEvent::Contact { .. } => lua_kinematic::CALLBACK_KINEMATIC_CONTACT,
    };

    emit_physics_local_callback(lua, game, LocalPhysicsCallback {
        entity: event.kinematic(),
        other: None,
        role: lua_kinematic::ROLE_KINEMATIC,
        callback_name,
        payload,
        event_label: "Kinematic",
        self_entity_key: lua_kinematic::EVENT_SELF_ENTITY,
        other_key: None,
        role_key: lua_kinematic::EVENT_ROLE,
    });
    emit_physics_local_callback(lua, game, LocalPhysicsCallback {
        entity: event.other(),
        other: None,
        role: lua_kinematic::ROLE_OTHER,
        callback_name,
        payload,
        event_label: "Kinematic",
        self_entity_key: lua_kinematic::EVENT_SELF_ENTITY,
        other_key: None,
        role_key: lua_kinematic::EVENT_ROLE,
    });
}

fn emit_collision_global_event(lua: &Lua, game: &Game, event: CollisionEvent) {
    let Ok(payload) = collision_event_payload(lua, event) else {
        omni_error!("Failed to build collision Lua event payload for {:?}", event);
        return;
    };

    game.script_manager.event_bus.emit(
        collision_event_name(event).to_string(),
        Variadic::from_iter([Value::Table(payload.clone())]),
    );
    emit_collision_local_callbacks(lua, game, event, &payload);
}

fn collision_event_payload(lua: &Lua, event: CollisionEvent) -> mlua::Result<Table> {
    let payload = lua.create_table()?;
    payload.set(lua_collision::EVENT_KIND, collision_event_kind(event))?;
    match event {
        CollisionEvent::Enter { first, second } | CollisionEvent::Exit { first, second } => {
            payload.set(lua_collision::EVENT_FIRST, lua_entity_handle(lua, first)?)?;
            payload.set(lua_collision::EVENT_SECOND, lua_entity_handle(lua, second)?)?;
        }
        CollisionEvent::Squeeze { kinematic, dynamic } => {
            payload.set(lua_collision::EVENT_KINEMATIC, lua_entity_handle(lua, kinematic)?)?;
            payload.set(lua_collision::EVENT_DYNAMIC, lua_entity_handle(lua, dynamic)?)?;
        }
    }
    Ok(payload)
}

fn emit_collision_local_callbacks(lua: &Lua, game: &Game, event: CollisionEvent, payload: &Table) {
    match event {
        CollisionEvent::Enter { first, second } | CollisionEvent::Exit { first, second } => {
            let callback_name = collision_callback_name(event);
            emit_physics_local_callback(lua, game, LocalPhysicsCallback {
                entity: first,
                other: Some(second),
                role: lua_collision::ROLE_FIRST,
                callback_name,
                payload,
                event_label: "Collision",
                self_entity_key: lua_collision::EVENT_SELF_ENTITY,
                other_key: Some(lua_collision::EVENT_OTHER),
                role_key: lua_collision::EVENT_ROLE,
            });
            emit_physics_local_callback(lua, game, LocalPhysicsCallback {
                entity: second,
                other: Some(first),
                role: lua_collision::ROLE_SECOND,
                callback_name,
                payload,
                event_label: "Collision",
                self_entity_key: lua_collision::EVENT_SELF_ENTITY,
                other_key: Some(lua_collision::EVENT_OTHER),
                role_key: lua_collision::EVENT_ROLE,
            });
        }
        CollisionEvent::Squeeze { kinematic, dynamic } => {
            let callback_name = collision_callback_name(event);
            emit_physics_local_callback(lua, game, LocalPhysicsCallback {
                entity: kinematic,
                other: Some(dynamic),
                role: lua_collision::ROLE_KINEMATIC,
                callback_name,
                payload,
                event_label: "Collision",
                self_entity_key: lua_collision::EVENT_SELF_ENTITY,
                other_key: Some(lua_collision::EVENT_OTHER),
                role_key: lua_collision::EVENT_ROLE,
            });
            emit_physics_local_callback(lua, game, LocalPhysicsCallback {
                entity: dynamic,
                other: Some(kinematic),
                role: lua_collision::ROLE_DYNAMIC,
                callback_name,
                payload,
                event_label: "Collision",
                self_entity_key: lua_collision::EVENT_SELF_ENTITY,
                other_key: Some(lua_collision::EVENT_OTHER),
                role_key: lua_collision::EVENT_ROLE,
            });
        }
    }
}

fn emit_sensor_global_event(lua: &Lua, game: &Game, event: SensorEvent) {
    let Ok(payload) = sensor_event_payload(lua, event) else {
        omni_error!("Failed to build sensor Lua event payload for {:?}", event);
        return;
    };

    game.script_manager.event_bus.emit(
        sensor_event_name(event).to_string(),
        Variadic::from_iter([Value::Table(payload.clone())]),
    );
    emit_sensor_local_callbacks(lua, game, event, &payload);
}

fn sensor_event_payload(lua: &Lua, event: SensorEvent) -> mlua::Result<Table> {
    let payload = lua.create_table()?;
    payload.set(lua_sensor::EVENT_SENSOR, lua_entity_handle(lua, event.sensor())?)?;
    payload.set(lua_sensor::EVENT_BODY, lua_entity_handle(lua, event.body())?)?;
    payload.set(lua_sensor::EVENT_KIND, sensor_event_kind(event))?;
    Ok(payload)
}

fn emit_sensor_local_callbacks(lua: &Lua, game: &Game, event: SensorEvent, payload: &Table) {
    let callback_name = sensor_callback_name(event);
    emit_physics_local_callback(lua, game, LocalPhysicsCallback {
        entity: event.sensor(),
        other: Some(event.body()),
        role: lua_sensor::ROLE_SENSOR,
        callback_name,
        payload,
        event_label: "Sensor",
        self_entity_key: lua_sensor::EVENT_SELF_ENTITY,
        other_key: Some(lua_sensor::EVENT_OTHER),
        role_key: lua_sensor::EVENT_ROLE,
    });
    emit_physics_local_callback(lua, game, LocalPhysicsCallback {
        entity: event.body(),
        other: Some(event.sensor()),
        role: lua_sensor::ROLE_BODY,
        callback_name,
        payload,
        event_label: "Sensor",
        self_entity_key: lua_sensor::EVENT_SELF_ENTITY,
        other_key: Some(lua_sensor::EVENT_OTHER),
        role_key: lua_sensor::EVENT_ROLE,
    });
}

fn emit_physics_local_callback(lua: &Lua, game: &Game, callback: LocalPhysicsCallback<'_>) {
    let Some((instance, func)) = lookup_entity_function(game, callback.entity, callback.callback_name) else {
        return;
    };
    let Ok(local_payload) = physics_local_payload(lua, &callback) else {
        omni_error!(
            "Failed to build {} local payload for {:?} on {:?}",
            callback.event_label,
            callback.callback_name,
            callback.entity
        );
        return;
    };

    if let Err(err) = func.call::<()>((instance, local_payload)) {
        omni_error!(
            "{} callback '{}' failed for {:?}: {}",
            callback.event_label,
            callback.callback_name,
            callback.entity,
            err
        );
    }
}

fn physics_local_payload(lua: &Lua, callback: &LocalPhysicsCallback<'_>) -> mlua::Result<Table> {
    let local_payload = lua.create_table()?;
    for pair in callback.payload.pairs::<String, Value>() {
        let (key, value) = pair?;
        local_payload.set(key, value)?;
    }
    local_payload.set(callback.self_entity_key, lua_entity_handle(lua, callback.entity)?)?;
    if let (Some(other), Some(other_key)) = (callback.other, callback.other_key) {
        local_payload.set(other_key, lua_entity_handle(lua, other)?)?;
    }
    local_payload.set(callback.role_key, callback.role)?;
    Ok(local_payload)
}

fn kinematic_event_name(event: KinematicContactEvent) -> &'static str {
    match event {
        KinematicContactEvent::Contact { .. } => lua_events::KINEMATIC_CONTACT,
    }
}

fn kinematic_event_kind(event: KinematicContactEvent) -> &'static str {
    match event {
        KinematicContactEvent::Contact { .. } => lua_kinematic::KIND_TRIGGER,
    }
}

fn collision_event_name(event: CollisionEvent) -> &'static str {
    match event {
        CollisionEvent::Enter { .. } => lua_events::COLLISION_ENTER,
        CollisionEvent::Exit { .. } => lua_events::COLLISION_EXIT,
        CollisionEvent::Squeeze { .. } => lua_events::COLLISION_SQUEEZE,
    }
}

fn collision_event_kind(event: CollisionEvent) -> &'static str {
    match event {
        CollisionEvent::Enter { .. } => lua_collision::KIND_ENTER,
        CollisionEvent::Exit { .. } => lua_collision::KIND_EXIT,
        CollisionEvent::Squeeze { .. } => lua_collision::KIND_SQUEEZE,
    }
}

fn collision_callback_name(event: CollisionEvent) -> &'static str {
    match event {
        CollisionEvent::Enter { .. } => lua_collision::CALLBACK_COLLISION_ENTER,
        CollisionEvent::Exit { .. } => lua_collision::CALLBACK_COLLISION_EXIT,
        CollisionEvent::Squeeze { .. } => lua_collision::CALLBACK_COLLISION_SQUEEZE,
    }
}

fn sensor_event_name(event: SensorEvent) -> &'static str {
    match event {
        SensorEvent::Enter { .. } => lua_events::SENSOR_ENTER,
        SensorEvent::Stay { .. } => lua_events::SENSOR_STAY,
        SensorEvent::Exit { .. } => lua_events::SENSOR_EXIT,
    }
}

fn sensor_event_kind(event: SensorEvent) -> &'static str {
    match event {
        SensorEvent::Enter { .. } => lua_sensor::KIND_ENTER,
        SensorEvent::Stay { .. } => lua_sensor::KIND_STAY,
        SensorEvent::Exit { .. } => lua_sensor::KIND_EXIT,
    }
}

fn sensor_callback_name(event: SensorEvent) -> &'static str {
    match event {
        SensorEvent::Enter { .. } => lua_sensor::CALLBACK_SENSOR_ENTER,
        SensorEvent::Stay { .. } => lua_sensor::CALLBACK_SENSOR_STAY,
        SensorEvent::Exit { .. } => lua_sensor::CALLBACK_SENSOR_EXIT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collision_event_name_when_enter_exit_and_squeeze_use_collision_namespace() {
        let first = Entity(1);
        let second = Entity(2);

        assert_eq!(
            collision_event_name(CollisionEvent::Enter { first, second }),
            lua_events::COLLISION_ENTER
        );
        assert_eq!(
            collision_event_name(CollisionEvent::Exit { first, second }),
            lua_events::COLLISION_EXIT
        );
        assert_eq!(
            collision_event_name(CollisionEvent::Squeeze {
                kinematic: first,
                dynamic: second,
            }),
            lua_events::COLLISION_SQUEEZE
        );
    }

    #[test]
    fn collision_callback_name_when_enter_exit_and_squeeze_use_collision_callbacks() {
        let first = Entity(1);
        let second = Entity(2);

        assert_eq!(
            collision_callback_name(CollisionEvent::Enter { first, second }),
            lua_collision::CALLBACK_COLLISION_ENTER
        );
        assert_eq!(
            collision_callback_name(CollisionEvent::Exit { first, second }),
            lua_collision::CALLBACK_COLLISION_EXIT
        );
        assert_eq!(
            collision_callback_name(CollisionEvent::Squeeze {
                kinematic: first,
                dynamic: second,
            }),
            lua_collision::CALLBACK_COLLISION_SQUEEZE
        );
    }

    #[test]
    fn collision_event_payload_when_variants_differ_contains_expected_fields() {
        let lua = Lua::new();
        let first = Entity(1);
        let second = Entity(2);
        let enter_payload = collision_event_payload(&lua, CollisionEvent::Enter { first, second })
            .unwrap();
        let squeeze_payload = collision_event_payload(&lua, CollisionEvent::Squeeze {
            kinematic: first,
            dynamic: second,
        }).unwrap();

        assert_eq!(
            enter_payload.get::<String>(lua_collision::EVENT_KIND).unwrap(),
            lua_collision::KIND_ENTER
        );
        assert!(enter_payload.get::<mlua::AnyUserData>(lua_collision::EVENT_FIRST).is_ok());
        assert!(enter_payload.get::<mlua::AnyUserData>(lua_collision::EVENT_SECOND).is_ok());
        assert!(enter_payload
            .get::<Option<mlua::AnyUserData>>(lua_collision::EVENT_KINEMATIC)
            .unwrap()
            .is_none());
        assert!(enter_payload
            .get::<Option<mlua::AnyUserData>>(lua_collision::EVENT_DYNAMIC)
            .unwrap()
            .is_none());

        assert_eq!(
            squeeze_payload.get::<String>(lua_collision::EVENT_KIND).unwrap(),
            lua_collision::KIND_SQUEEZE
        );
        assert!(squeeze_payload.get::<mlua::AnyUserData>(lua_collision::EVENT_KINEMATIC).is_ok());
        assert!(squeeze_payload.get::<mlua::AnyUserData>(lua_collision::EVENT_DYNAMIC).is_ok());
        assert!(squeeze_payload
            .get::<Option<mlua::AnyUserData>>(lua_collision::EVENT_FIRST)
            .unwrap()
            .is_none());
        assert!(squeeze_payload
            .get::<Option<mlua::AnyUserData>>(lua_collision::EVENT_SECOND)
            .unwrap()
            .is_none());
    }
}
