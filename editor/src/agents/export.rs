use crate::app::Editor;
use engine_core::agents::payload::{AgentBuiltPayload, AgentPayloadError, AgentPayloadSpec};
use engine_core::ecs::component::CurrentRoom;
use engine_core::ecs::component_registry::ComponentRegistry;
use engine_core::ecs::entity::Entity;
use engine_core::scripting::script::Script;
use engine_core::worlds::room::RoomId;
use ron::ser::PrettyConfig;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

pub fn build_seeded_agent_payload(
    editor: &Editor,
    room_id: RoomId,
) -> Result<AgentBuiltPayload, AgentPayloadError> {
    let room = editor
        .game
        .current_world()
        .get_room(room_id)
        .cloned()
        .ok_or(AgentPayloadError::MissingRoom)?;
    let mut spec = AgentPayloadSpec::seeded(editor.game.name.clone()).add_room(room.name.clone());

    let known_registry_ids: HashSet<_> = inventory::iter::<ComponentRegistry>
        .into_iter()
        .map(|registry| registry.type_id)
        .collect();

    let registered_store_name = |store_id: &std::any::TypeId| {
        inventory::iter::<ComponentRegistry>
            .into_iter()
            .find(|registry| registry.type_id == *store_id)
            .map(|registry| registry.type_name.to_string())
    };

    if editor
        .game
        .ecs
        .stores
        .keys()
        .any(|store_id| !known_registry_ids.contains(store_id))
    {
        return Err(AgentPayloadError::UnknownComponentType(
            editor
                .game
                .ecs
                .stores
                .keys()
                .find(|store_id| !known_registry_ids.contains(store_id))
                .and_then(registered_store_name)
                .unwrap_or_else(|| "unknown component store".to_string()),
        ));
    }

    let entity_ids: Vec<Entity> = editor
        .game
        .ecs
        .get_store::<CurrentRoom>()
        .data
        .iter()
        .filter_map(|(&entity, current_room)| (current_room.0 == room_id).then_some(entity))
        .collect();

    let mut seen_names = HashSet::new();

    if entity_ids.is_empty() {
        return Err(AgentPayloadError::MissingEntity(format!(
            "room {:?}",
            room_id
        )));
    }

    for entity in entity_ids {
        let name = editor
            .game
            .ecs
            .get::<engine_core::ecs::component::Name>(entity)
            .map(|name| name.0.clone())
            .unwrap_or_else(|| format!("Entity{}", entity.0));

        if !seen_names.insert(name.clone()) {
            return Err(AgentPayloadError::DuplicateEntityName(name));
        }

        spec = spec.add_entity(name.clone());

        for registry in inventory::iter::<ComponentRegistry> {
            if !(registry.has)(&editor.game.ecs, entity) {
                continue;
            }

            let boxed = (registry.clone)(&editor.game.ecs, entity);
            let ron = (registry.to_ron_component)(&*boxed);
            spec = spec.attach_component(&name, registry.type_name, &ron);
        }

        if let Some(script) = editor.game.ecs.get::<Script>(entity) {
            let path = editor
                .game
                .script_manager
                .script_id_to_path
                .get(&script.script_id)
                .ok_or_else(|| {
                    AgentPayloadError::UnknownScriptType(
                        editor
                            .game
                            .script_manager
                            .path_to_script_id
                            .iter()
                            .find(|(_, id)| **id == script.script_id)
                            .map(|(path, _)| path.to_string_lossy().to_string())
                            .unwrap_or_else(|| script.script_id.0.to_string()),
                    )
                })?;

            spec = spec.attach_script(&name, path.to_string_lossy().as_ref());
        }
    }

    let mut built = spec.build()?;
    built.room = room.clone();
    let Some(world) = built.game.worlds.first_mut() else {
        return Ok(built);
    };
    let Some(built_room) = world.rooms.first_mut() else {
        return Ok(built);
    };
    *built_room = room;
    for entity in &mut built.entities {
        entity.room_id = built_room.id;
    }
    world.current_room_id = Some(built_room.id);
    world.starting_room_id = Some(built_room.id);
    Ok(built)
}

pub fn write_seeded_agent_payload(
    editor: &Editor,
    room_id: RoomId,
) -> Result<PathBuf, AgentPayloadError> {
    let payload = build_seeded_agent_payload(editor, room_id)?;
    let ron = ron::ser::to_string_pretty(&payload, PrettyConfig::default())
        .map_err(|error| AgentPayloadError::SerializePayload(error.to_string()))?;

    let mut path = std::env::temp_dir();
    path.push(format!("seeded_agent_payload_{}.ron", Uuid::new_v4()));
    fs::write(&path, ron).map_err(|error| AgentPayloadError::WritePayload(error.to_string()))?;
    Ok(path)
}
