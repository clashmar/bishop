use super::ResourceSource;
use crate::constants::paths;
use crate::ecs::{CurrentRoom, Ecs, Global};
use crate::game::{Game, GameDataManifest};
use crate::storage::path_utils::{room_payload_path, world_descriptor_path};
use crate::worlds::{Room, RoomId, RoomPayload, World, WorldDescriptor};
use serde::de::DeserializeOwned;
use std::io::{self, ErrorKind};
use std::path::Path;

/// Loads descriptor shells, ECS and registries from one authored source.
pub fn load_game_shell_from_source(source: &ResourceSource) -> io::Result<Game> {
    let manifest: GameDataManifest = read_ron(source, Path::new(paths::GAME_RON))?;
    let mut game = Game::with_name(manifest.name);
    game.version = manifest.version;
    game.current_world_id = manifest.current_world_id;
    game.runtime_source = Some(source.clone());

    for world_id in manifest.world_ids {
        let descriptor: WorldDescriptor = read_ron(source, &world_descriptor_path(Path::new(""), world_id))?;
        game.add_world(World::from_descriptor(descriptor));
    }
    game.rebuild_world_index();

    if let Some(ecs) = read_optional_ron(source, Path::new(paths::ECS_RON))? {
        game.ecs = ecs;
    }
    if let Some(registry) = read_optional_ron(source, Path::new(paths::ASSET_REGISTRY_RON))? {
        game.asset_registry = registry;
    }
    if let Some(registry) = read_optional_ron(source, Path::new(paths::TILE_REGISTRY_RON))? {
        game.tile_registry = registry;
    }
    validate_global_room_membership(&game.ecs)?;
    Ok(game)
}

/// Hydrates the current world's active room from the same source as its shell.
pub fn hydrate_current_payloads_from_source(source: &ResourceSource, game: &mut Game) -> io::Result<()> {
    let Some(world) = game.current_world_mut() else { return Ok(()) };
    let Some(room) = world.current_room_mut() else { return Ok(()) };
    hydrate_room_payload_from_source(source, room)
}

/// Hydrates an authored room by id from a game's retained source.
pub fn hydrate_room_from_source(source: &ResourceSource, game: &mut Game, room_id: RoomId) -> io::Result<()> {
    let world_id = game.room_world_map().get(&room_id).copied().ok_or_else(|| {
        io::Error::new(ErrorKind::NotFound, format!("no world owns room payload Room({})", room_id.0))
    })?;
    let room = game.get_world_mut(world_id).and_then(|world| world.get_room_mut(room_id))
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, format!("room payload Room({}) is missing", room_id.0)))?;
    hydrate_room_payload_from_source(source, room)
}

pub(crate) fn hydrate_all_payloads_from_source(source: &ResourceSource, game: &mut Game) -> io::Result<()> {
    for world in game.worlds_mut() {
        for room in world.rooms_mut() {
            hydrate_room_payload_from_source(source, room)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_global_room_membership(ecs: &Ecs) -> io::Result<()> {
    for &entity in ecs.get_store::<Global>().data.keys() {
        if ecs.has::<CurrentRoom>(entity) {
            return Err(io::Error::other(format!(
                "global entity {entity:?} must not have a CurrentRoom; placement belongs on a player proxy"
            )));
        }
    }
    Ok(())
}

fn hydrate_room_payload_from_source(source: &ResourceSource, room: &mut Room) -> io::Result<()> {
    let payload: RoomPayload = read_ron(source, &room_payload_path(Path::new(""), room.id))?;
    payload.apply(room);
    Ok(())
}

fn read_ron<T: DeserializeOwned>(source: &ResourceSource, path: &Path) -> io::Result<T> {
    let bytes = source.read_required(path)?;
    parse_ron(&bytes, path)
}

fn read_optional_ron<T: DeserializeOwned>(source: &ResourceSource, path: &Path) -> io::Result<Option<T>> {
    source.read_optional(path)?.map(|bytes| parse_ron(&bytes, path)).transpose()
}

fn parse_ron<T: DeserializeOwned>(bytes: &[u8], path: &Path) -> io::Result<T> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        io::Error::new(ErrorKind::InvalidData, format!("invalid UTF-8 in {}: {error}", path.display()))
    })?;
    ron::from_str(text).map_err(|error| {
        io::Error::new(ErrorKind::InvalidData, format!("invalid RON in {}: {error}", path.display()))
    })
}
