use crate::constants::paths;
use crate::game::{Game, GameDataManifest};
use crate::storage::path_utils::{
    resources_folder, room_payload_path, world_descriptor_path,
};
use crate::worlds::{RoomPayload, World, WorldDescriptor};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Saves a `Game` into the split-layout format under `folder`.
pub fn save_game_to_folder(game: &Game, folder: &Path) -> io::Result<()> {
    fs::create_dir_all(folder)?;

    // Save the full ECS data so entities round-trip
    let ecs_ron = ron::ser::to_string_pretty(&game.ecs, ron::ser::PrettyConfig::new())
        .map_err(io::Error::other)?;
    fs::write(folder.join(paths::ECS_RON), ecs_ron)?;

    // Save the asset registry
    let registry_ron = ron::ser::to_string_pretty(&game.asset_registry, ron::ser::PrettyConfig::new())
        .map_err(io::Error::other)?;
    fs::write(folder.join(paths::ASSET_REGISTRY_RON), registry_ron)?;

    // Build and write the game-data manifest
    let mut world_ids = Vec::new();
    let worlds_folder = folder.join(paths::WORLDS_FOLDER);
    let payloads_folder = folder.join(paths::PAYLOADS_FOLDER);
    let mut expected_world_files = Vec::new();
    let mut expected_payload_files = Vec::new();
    fs::create_dir_all(&worlds_folder)?;
    fs::create_dir_all(&payloads_folder)?;

    for world in game.worlds() {
        let descriptor_path = world_descriptor_path(folder, world.id);
        expected_world_files.push(descriptor_path.clone());

        let mut room_entries = Vec::new();
        for room in world.rooms() {
            let payload_path = room_payload_path(folder, room.id);
            let payload_ron = ron::ser::to_string_pretty(
                &RoomPayload::capture(game, room),
                ron::ser::PrettyConfig::new(),
            )
            .map_err(io::Error::other)?;
            fs::write(&payload_path, payload_ron)?;
            expected_payload_files.push(payload_path);

            room_entries.push(crate::worlds::RoomDirectoryEntry {
                id: room.id,
                name: room.name.clone(),
                position: room.position,
                size: room.size,
                exits: room.exits.clone(),
                adjacent_rooms: room.adjacent_rooms.clone(),
                tags: room.tags.clone(),
                singleton: room.singleton,
            });
        }

        let descriptor = WorldDescriptor {
            id: world.id,
            name: world.name.clone(),
            current_room_id: world.current_room_id,
            meta: world.meta.clone(),
            tags: world.tags.clone(),
            overlay: world.overlay,
            grid_size: world.grid_size,
            singleton: world.singleton,
            rooms: room_entries,
        };
        let descriptor_ron =
            ron::ser::to_string_pretty(&descriptor, ron::ser::PrettyConfig::new())
                .map_err(io::Error::other)?;
        fs::write(descriptor_path, descriptor_ron)?;
        world_ids.push(world.id);
    }

    let manifest = GameDataManifest {
        version: game.version,
        name: game.name.clone(),
        current_world_id: game.current_world_id,
        world_ids,
    };
    let manifest_ron =
        ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::new())
            .map_err(io::Error::other)?;
    fs::write(folder.join(paths::GAME_RON), manifest_ron)?;
    remove_stale_split_layout_files(&worlds_folder, &expected_world_files)?;
    remove_stale_split_layout_files(&payloads_folder, &expected_payload_files)?;

    Ok(())
}

fn remove_stale_split_layout_files(layout_dir: &Path, expected_files: &[PathBuf]) -> io::Result<()> {
    let expected_paths: HashSet<_> = expected_files.iter().cloned().collect();

    for entry in fs::read_dir(layout_dir)? {
        let path = entry?.path();
        if path.is_file()
            && path.extension().is_some_and(|extension| extension == "ron")
            && !expected_paths.contains(&path)
        {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

/// Loads descriptor shells plus shared ECS/registry state from a game folder.
/// Room payload data remain unloaded until a later hydration step.
pub fn load_game_shell_from_folder(folder: &Path) -> io::Result<Game> {
    let manifest_path = folder.join(paths::GAME_RON);
    let manifest_ron = fs::read_to_string(&manifest_path)?;
    let manifest: GameDataManifest = ron::from_str(&manifest_ron).map_err(io::Error::other)?;

    load_shell_from_split_layout(folder, manifest)
}

/// Hydrates every split-layout payload back into a fully materialized `Game`.
pub fn load_full_game_from_folder(folder: &Path) -> io::Result<Game> {
    let mut game = load_game_shell_from_folder(folder)?;
    hydrate_all_payloads_from_folder(folder, &mut game)?;
    game.ecs.finalize_after_load();
    Ok(game)
}

/// Hydrates the current world's startup-critical payloads after descriptor-shell boot.
pub fn hydrate_current_payloads_from_folder(folder: &Path, game: &mut Game) -> io::Result<()> {
    hydrate_current_room_payload(folder, game)?;
    Ok(())
}

/// Hydrates the initial payloads needed for runtime after a descriptor-shell boot.
pub fn hydrate_initial_payloads_for_runtime(game: &mut Game) -> Result<(), String> {
    let resources = resources_folder(&game.name);
    if !resources.join(paths::GAME_RON).exists() {
        return Ok(());
    }

    if game
        .current_world()
        .current_room()
        .is_none_or(|room| !room.variants.is_empty())
    {
        return Ok(());
    }

    hydrate_current_payloads_from_folder(&resources, game)
        .map_err(|error| format!("Failed to hydrate initial payloads: {error}"))
}

fn load_shell_from_split_layout(folder: &Path, manifest: GameDataManifest) -> io::Result<Game> {
    let mut game = Game::with_name(manifest.name);
    game.version = manifest.version;
    game.current_world_id = manifest.current_world_id;

    for world_id in manifest.world_ids {
        let descriptor_ron = fs::read_to_string(world_descriptor_path(folder, world_id))?;
        let descriptor: WorldDescriptor =
            ron::from_str(&descriptor_ron).map_err(io::Error::other)?;
        game.add_world(World::from_descriptor(descriptor));
    }

    game.rebuild_world_index();

    let ecs_path = folder.join(paths::ECS_RON);
    if ecs_path.exists() {
        let ecs_ron = fs::read_to_string(&ecs_path)?;
        game.ecs = ron::from_str(&ecs_ron).map_err(io::Error::other)?;
    }

    let registry_path = folder.join(paths::ASSET_REGISTRY_RON);
    if registry_path.exists() {
        let registry_ron = fs::read_to_string(&registry_path)?;
        game.asset_registry = ron::from_str(&registry_ron).map_err(io::Error::other)?;
    }

    Ok(game)
}

fn hydrate_all_payloads_from_folder(folder: &Path, game: &mut Game) -> io::Result<()> {
    for world in game.worlds_mut() {
        for room in world.rooms_mut() {
            hydrate_room_payload(folder, room)?;
        }
    }
    Ok(())
}

fn hydrate_current_room_payload(folder: &Path, game: &mut Game) -> io::Result<()> {
    let Some(world) = game.current_world_mut() else {
        return Ok(());
    };
    let Some(room) = world.current_room_mut() else {
        return Ok(());
    };
    hydrate_room_payload(folder, room)
}

fn hydrate_room_payload(folder: &Path, room: &mut crate::worlds::Room) -> io::Result<()> {
    let payload_ron = fs::read_to_string(room_payload_path(folder, room.id))?;
    let payload: RoomPayload = ron::from_str(&payload_ron).map_err(io::Error::other)?;
    payload.apply(room);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{Animation, Entity};
    use crate::worlds::{Room, RoomId, WorldId};
    use std::path::PathBuf;

    struct TempSplitLayoutDir(PathBuf);

    impl TempSplitLayoutDir {
        fn new() -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!("bishop_split_layout_{unique}")))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempSplitLayoutDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fully_loaded_test_game() -> Game {
        let mut game = Game::default();
        let mut world = World::new(WorldId(1), "Demo".to_string(), 16.0);
        world.current_room_id = Some(RoomId(1));
        let room = Room::new(&mut game.ecs, RoomId(1), 16.0);
        world.add_room(room);
        world.singleton = game
            .ecs
            .create_entity()
            .with(Animation::default())
            .with_current_room(RoomId(1))
            .finish();
        game.add_world(world);
        game.ecs
            .create_entity()
            .with(Animation::default())
            .with_current_room(RoomId(1))
            .finish();
        game
    }

    fn deterministic_save_test_game() -> (Game, Vec<Entity>) {
        let mut game = fully_loaded_test_game();
        let room_id = RoomId(1);
        game.ecs
            .create_entity()
            .with(Animation::default())
            .with_current_room(room_id)
            .finish();
        game.ecs
            .create_entity()
            .with(Animation::default())
            .with_current_room(room_id)
            .finish();
        game.ecs
            .create_entity()
            .with(Animation::default())
            .with_current_room(room_id)
            .finish();

        let mut expected_room_entities: Vec<_> =
            game.ecs.entities_in_room(room_id).iter().copied().collect();
        expected_room_entities.sort_unstable();
        (game, expected_room_entities)
    }

    fn fully_loaded_multi_world_test_game() -> Game {
        let mut game = Game::default();

        let mut first_world = World::new(WorldId(1), "First".to_string(), 16.0);
        first_world.current_room_id = Some(RoomId(1));
        first_world.add_room(Room::new(&mut game.ecs, RoomId(1), 16.0));
        first_world.singleton = game
            .ecs
            .create_entity()
            .with(Animation::default())
            .with_current_room(RoomId(1))
            .finish();
        game.add_world(first_world);

        let mut second_world = World::new(WorldId(2), "Second".to_string(), 16.0);
        second_world.current_room_id = Some(RoomId(2));
        second_world.add_room(Room::new(&mut game.ecs, RoomId(2), 16.0));
        second_world.singleton = game
            .ecs
            .create_entity()
            .with(Animation::default())
            .with_current_room(RoomId(2))
            .finish();
        game.add_world(second_world);
        game.current_world_id = Some(WorldId(2));

        game
    }

    fn fully_loaded_two_room_test_game() -> Game {
        let mut game = Game::default();
        let mut world = World::new(WorldId(1), "Demo".to_string(), 16.0);
        world.current_room_id = Some(RoomId(1));
        world.add_room(Room::new(&mut game.ecs, RoomId(1), 16.0));
        world.add_room(Room::new(&mut game.ecs, RoomId(2), 16.0));
        world.singleton = game
            .ecs
            .create_entity()
            .with(Animation::default())
            .with_current_room(RoomId(1))
            .finish();
        game.add_world(world);
        game
    }

    #[test]
    fn save_game_to_folder_writes_manifest_descriptors_and_payloads() {
        let folder = TempSplitLayoutDir::new();
        let game = fully_loaded_test_game();

        save_game_to_folder(&game, folder.path()).unwrap();

        assert!(folder.path().join(paths::GAME_RON).is_file());
        assert!(
            folder
                .path()
                .join(paths::WORLDS_FOLDER)
                .join("world-1.ron")
                .is_file()
        );
        assert!(
            folder
                .path()
                .join(paths::PAYLOADS_FOLDER)
                .join("room-1.ron")
                .is_file()
        );
    }

    #[test]
    fn load_game_shell_from_folder_defers_room_payloads() {
        let folder = TempSplitLayoutDir::new();
        let original = fully_loaded_test_game();
        save_game_to_folder(&original, folder.path()).unwrap();

        let loaded = load_game_shell_from_folder(folder.path()).unwrap();
        let room = &loaded.current_world().rooms()[0];

        assert!(room.variants.is_empty());
    }

    #[test]
    fn hydrate_current_payloads_from_folder_restores_current_room_variants() {
        let folder = TempSplitLayoutDir::new();
        let original = fully_loaded_test_game();
        save_game_to_folder(&original, folder.path()).unwrap();

        let mut loaded = load_game_shell_from_folder(folder.path()).unwrap();
        hydrate_current_payloads_from_folder(folder.path(), &mut loaded).unwrap();

        assert!(!loaded.current_world().current_room().unwrap().variants.is_empty());
    }

    #[test]
    fn load_full_game_from_folder_rebuilds_indexes_from_split_layout() {
        let folder = TempSplitLayoutDir::new();
        let original = fully_loaded_test_game();
        save_game_to_folder(&original, folder.path()).unwrap();

        let loaded = load_full_game_from_folder(folder.path()).unwrap();
        let room_id = original.current_world().rooms()[0].id;

        assert!(loaded.current_world().get_room(room_id).is_some());
        assert!(!loaded.ecs.entities_in_room(room_id).is_empty());
    }

    #[test]
    fn load_game_shell_from_folder_preserves_world_and_room_singletons() {
        let folder = TempSplitLayoutDir::new();
        let original = fully_loaded_test_game();
        let room_id = original.current_world().rooms()[0].id;
        let room_singleton = original.current_world().get_room(room_id).unwrap().singleton;
        let world_singleton = original.current_world().singleton;
        save_game_to_folder(&original, folder.path()).unwrap();

        let loaded = load_game_shell_from_folder(folder.path()).unwrap();

        assert_eq!(loaded.current_world().singleton, world_singleton);
        assert_eq!(loaded.current_world().get_room(room_id).unwrap().singleton, room_singleton);
    }

    #[test]
    fn save_game_to_folder_writes_deterministic_ecs_and_sorted_room_payload_entities() {
        let folder = TempSplitLayoutDir::new();
        let (game, expected_room_entities) = deterministic_save_test_game();
        let (other_game, _) = deterministic_save_test_game();

        let ecs_ron = ron::ser::to_string_pretty(&game.ecs, ron::ser::PrettyConfig::new()).unwrap();
        let other_ecs_ron =
            ron::ser::to_string_pretty(&other_game.ecs, ron::ser::PrettyConfig::new()).unwrap();
        assert_eq!(ecs_ron, other_ecs_ron);

        save_game_to_folder(&game, folder.path()).unwrap();
        let room_payload_ron = fs::read_to_string(
            folder
                .path()
                .join(paths::PAYLOADS_FOLDER)
                .join("room-1.ron"),
        )
        .unwrap();
        let room_payload: RoomPayload = ron::from_str(&room_payload_ron).unwrap();

        assert_eq!(room_payload.entities, expected_room_entities);
    }

    #[test]
    fn save_game_to_folder_when_worlds_removed_deletes_stale_world_and_room_files() {
        let folder = TempSplitLayoutDir::new();
        let original = fully_loaded_multi_world_test_game();
        save_game_to_folder(&original, folder.path()).unwrap();

        assert!(folder.path().join(paths::WORLDS_FOLDER).join("world-2.ron").is_file());
        assert!(folder.path().join(paths::PAYLOADS_FOLDER).join("room-2.ron").is_file());

        let trimmed = fully_loaded_test_game();
        save_game_to_folder(&trimmed, folder.path()).unwrap();

        assert!(!folder.path().join(paths::WORLDS_FOLDER).join("world-2.ron").exists());
        assert!(!folder.path().join(paths::PAYLOADS_FOLDER).join("room-2.ron").exists());
    }

    #[test]
    fn save_game_to_folder_when_rooms_removed_deletes_stale_room_payload_files() {
        let folder = TempSplitLayoutDir::new();
        let original = fully_loaded_two_room_test_game();
        save_game_to_folder(&original, folder.path()).unwrap();

        assert!(folder.path().join(paths::PAYLOADS_FOLDER).join("room-2.ron").is_file());

        let trimmed = fully_loaded_test_game();
        save_game_to_folder(&trimmed, folder.path()).unwrap();

        assert!(!folder.path().join(paths::PAYLOADS_FOLDER).join("room-2.ron").exists());
    }

    #[test]
    fn load_full_game_from_folder_preserves_current_world_and_room_singletons() {
        let folder = TempSplitLayoutDir::new();
        let original = fully_loaded_multi_world_test_game();
        let world_singleton = original.current_world().singleton;
        let room_singleton = original.current_world().current_room().unwrap().singleton;
        save_game_to_folder(&original, folder.path()).unwrap();

        let loaded = load_full_game_from_folder(folder.path()).unwrap();
        let world = loaded.current_world();
        let room = world.current_room().unwrap();

        assert_eq!(loaded.current_world_id, Some(WorldId(2)));
        assert_eq!(world.singleton, world_singleton);
        assert_eq!(room.singleton, room_singleton);
        assert_ne!(world.singleton, Entity::default());
        assert_ne!(room.singleton, Entity::default());
    }
}
