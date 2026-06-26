use crate::assets::AssetKey;
use crate::audio::loader::sound_id_from_asset_path;
use crate::audio::AudioManager;
use crate::game::Game;
use crate::hydration::coordinator::HydrationCoordinator;
use crate::hydration::residency_key::{PayloadKey, ResidencyKey};
use crate::hydration::scope::{HydrationScope, ResourceClass};
use crate::storage::path_utils::{resources_folder, room_payload_path, world_payload_path};
use crate::worlds::{RoomPayload, WorldPayload};
use bishop::prelude::TextureLoader;
use mlua::Lua;
use std::fs;

/// Errors surfaced while hydrating claimed resources.
#[derive(Debug)]
pub enum HydrationError {
    UnknownAudioAssetPath(AssetKey),
    TextureLoad(String),
    ScriptLoad(mlua::Error),
    PayloadLoad(String),
}

/// Bridges coordinator residency claims into payload and asset load/release APIs.
pub struct HydrationDriver<'a> {
    pub game: &'a mut Game,
    pub audio_manager: &'a mut AudioManager,
}

impl<'a> HydrationDriver<'a> {
    /// Hydrates one residency key immediately using an explicit texture loader.
    pub fn hydrate_key(
        &mut self,
        key: ResidencyKey,
        texture_loader: &impl TextureLoader,
        lua: &Lua,
    ) -> Result<(), HydrationError> {
        match key {
            ResidencyKey::Asset(asset) => self.hydrate_asset(asset, texture_loader, lua),
            ResidencyKey::Payload(payload) => self.hydrate_payload(payload),
        }
    }

    /// Hydrates one asset immediately using an explicit texture loader.
    pub fn hydrate_asset(
        &mut self,
        asset: AssetKey,
        texture_loader: &impl TextureLoader,
        lua: &Lua,
    ) -> Result<(), HydrationError> {
        match asset {
            AssetKey::Sprite(id) => self
                .game
                .sprite_manager
                .ensure_loaded(texture_loader, id)
                .map_err(HydrationError::TextureLoad),
            AssetKey::Script(id) => self
                .game
                .script_manager
                .load_script_table(lua, id)
                .map(|_| ())
                .map_err(HydrationError::ScriptLoad),
            AssetKey::Sound(_) => {
                let Some(path) = self
                    .game
                    .asset_registry
                    .record(asset)
                    .map(|record| &record.path)
                else {
                    return Err(HydrationError::UnknownAudioAssetPath(asset));
                };
                let Some(sound_id) = sound_id_from_asset_path(path) else {
                    return Err(HydrationError::UnknownAudioAssetPath(asset));
                };
                self.audio_manager.claim_sound(&sound_id);
                Ok(())
            }
            AssetKey::Prefab(_) | AssetKey::Toml(_) => Ok(()),
        }
    }

    /// Hydrates one residency key for runtime traversal warming.
    pub fn hydrate_key_runtime(&mut self, key: ResidencyKey, lua: &Lua) -> Result<(), HydrationError> {
        match key {
            ResidencyKey::Asset(asset) => self.hydrate_asset_runtime(asset, lua),
            ResidencyKey::Payload(payload) => self.hydrate_payload(payload),
        }
    }

    /// Hydrates one asset for runtime traversal warming.
    pub fn hydrate_asset_runtime(&mut self, asset: AssetKey, lua: &Lua) -> Result<(), HydrationError> {
        match asset {
            AssetKey::Sprite(id) => {
                self.game.sprite_manager.prewarm_runtime_texture(id);
                Ok(())
            }
            AssetKey::Script(id) => self
                .game
                .script_manager
                .load_script_table(lua, id)
                .map(|_| ())
                .map_err(HydrationError::ScriptLoad),
            AssetKey::Sound(_) => {
                let Some(path) = self
                    .game
                    .asset_registry
                    .record(asset)
                    .map(|record| &record.path)
                else {
                    return Err(HydrationError::UnknownAudioAssetPath(asset));
                };
                let Some(sound_id) = sound_id_from_asset_path(path) else {
                    return Err(HydrationError::UnknownAudioAssetPath(asset));
                };
                self.audio_manager.claim_sound(&sound_id);
                Ok(())
            }
            AssetKey::Prefab(_) | AssetKey::Toml(_) => Ok(()),
        }
    }

    /// Releases one residency key.
    pub fn dehydrate_key(&mut self, key: ResidencyKey) {
        match key {
            ResidencyKey::Asset(asset) => self.dehydrate_asset(asset),
            ResidencyKey::Payload(payload) => self.dehydrate_payload(payload),
        }
    }

    /// Releases one hydrated asset.
    pub fn dehydrate_asset(&mut self, asset: AssetKey) {
        match asset {
            AssetKey::Sprite(id) => self.game.sprite_manager.evict_texture(id),
            AssetKey::Script(id) => self.game.script_manager.evict_script(id),
            AssetKey::Sound(_) => {
                if let Some(path) = self
                    .game
                    .asset_registry
                    .record(asset)
                    .map(|record| &record.path)
                    && let Some(sound_id) = sound_id_from_asset_path(path)
                {
                    self.audio_manager.release_claimed_sound(&sound_id);
                }
            }
            AssetKey::Prefab(_) | AssetKey::Toml(_) => {}
        }
    }

    /// Hydrates all claimed residency keys for one scope.
    pub fn hydrate_scope(
        &mut self,
        coordinator: &HydrationCoordinator,
        scope: &HydrationScope,
        texture_loader: &impl TextureLoader,
        lua: &Lua,
    ) -> Result<(), HydrationError> {
        let mut keys = coordinator.claimed_keys(scope);
        keys.sort_by_key(|key| {
            ResourceClass::for_residency_key(*key)
                .map(ResourceClass::hydration_priority)
                .unwrap_or(u8::MAX)
        });
        for key in keys {
            self.hydrate_key(key, texture_loader, lua)?;
        }
        Ok(())
    }

    /// Releases all claimed residency keys for one scope.
    pub fn dehydrate_scope(&mut self, coordinator: &HydrationCoordinator, scope: &HydrationScope) {
        let mut keys = coordinator.claimed_keys(scope);
        keys.sort_by_key(|key| {
            ResourceClass::for_residency_key(*key)
                .map(ResourceClass::dehydration_priority)
                .unwrap_or(u8::MAX)
        });
        for key in keys {
            self.dehydrate_key(key);
        }
    }

    fn hydrate_payload(&mut self, payload: PayloadKey) -> Result<(), HydrationError> {
        match payload {
            PayloadKey::Global => Ok(()),
            PayloadKey::World(world_id) => {
                let payload_path = world_payload_path(&resources_folder(&self.game.name), world_id);
                let payload_ron = fs::read_to_string(&payload_path)
                    .map_err(|error| {
                        HydrationError::PayloadLoad(format!(
                            "failed to read world payload '{}': {error}",
                            payload_path.display()
                        ))
                    })?;
                let payload: WorldPayload = ron::from_str(&payload_ron).map_err(|error| {
                    HydrationError::PayloadLoad(format!(
                        "failed to parse world payload '{}': {error}",
                        payload_path.display()
                    ))
                })?;

                if let Some(world) = self.game.get_world_mut(world_id) {
                    payload.apply(world);
                }
                Ok(())
            }
            PayloadKey::Room(room_id) => {
                let room_world_map = self.game.room_world_map();
                let Some(world_id) = room_world_map.get(&room_id).copied() else {
                    return Err(HydrationError::PayloadLoad(format!(
                        "no world owns room payload Room({})",
                        room_id.0
                    )));
                };
                let payload_path = room_payload_path(&resources_folder(&self.game.name), room_id);
                let payload_ron = fs::read_to_string(&payload_path)
                    .map_err(|error| {
                        HydrationError::PayloadLoad(format!(
                            "failed to read room payload '{}': {error}",
                            payload_path.display()
                        ))
                    })?;
                let payload: RoomPayload = ron::from_str(&payload_ron).map_err(|error| {
                    HydrationError::PayloadLoad(format!(
                        "failed to parse room payload '{}': {error}",
                        payload_path.display()
                    ))
                })?;

                if let Some(world) = self.game.get_world_mut(world_id)
                    && let Some(room) = world.get_room_mut(room_id)
                {
                    payload.apply(room);
                }
                Ok(())
            }
        }
    }

    fn dehydrate_payload(&mut self, payload: PayloadKey) {
        match payload {
            PayloadKey::Global => {}
            PayloadKey::World(world_id) => {
                if let Some(world) = self.game.get_world_mut(world_id) {
                    WorldPayload::clear(world);
                }
            }
            PayloadKey::Room(room_id) => {
                let room_world_map = self.game.room_world_map();
                let Some(world_id) = room_world_map.get(&room_id).copied() else {
                    return;
                };
                if let Some(world) = self.game.get_world_mut(world_id)
                    && let Some(room) = world.get_room_mut(room_id)
                {
                    RoomPayload::clear(room);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::test_utils::{CountingFailingLoader, TestBackend};
    use crate::ecs::{SoundId, SpriteId};
    use crate::engine_global::set_game_name;
    use crate::hydration::Hydratable;
    use crate::storage::{game_folder, load_game_shell_from_folder, save_game_to_folder};
    use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use crate::task::FileReadPool;
    use crate::worlds::{Room, RoomId, World, WorldId};
    use std::fs;

    #[test]
    fn hydrate_scope_attempts_sprite_assets() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let folder = TestGameFolder::new("hydration_driver_sprites");
        set_game_name(folder.name());
        fs::create_dir_all(crate::storage::path_utils::assets_folder().join("sprites")).unwrap();
        fs::write(
            crate::storage::path_utils::assets_folder().join("sprites/player.png"),
            [1_u8, 2, 3, 4],
        )
        .unwrap();

        let lua = Lua::new();
        let mut game = Game::with_name(folder.name());
        game.asset_registry
            .register_asset_relative_path(SpriteId(1), "sprites/player.png")
            .unwrap();
        crate::assets::SpriteManager::init_editor_metadata(
            &game.asset_registry,
            &mut game.sprite_manager,
        );
        let mut audio_manager = AudioManager::new::<TestBackend>();
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(2));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(1))));
        let loader = CountingFailingLoader::new();

        let mut driver = HydrationDriver {
            game: &mut game,
            audio_manager: &mut audio_manager,
        };

        let error = driver
            .hydrate_scope(&coordinator, &scope, &loader, &lua)
            .unwrap_err();
        assert!(matches!(error, HydrationError::TextureLoad(_)));
        assert_eq!(loader.load_calls.get(), 1);
    }

    #[test]
    fn hydrate_scope_hydrates_payload_before_asset_claims() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let folder = TestGameFolder::new("hydration_driver_payload_order");
        set_game_name(folder.name());

        let resources_dir = game_folder(folder.name()).join(crate::constants::paths::RESOURCES_FOLDER);
        let mut authored = Game::with_name(folder.name());
        let mut world = World::new(WorldId(1), "Demo".to_string(), 16.0);
        world.current_room_id = Some(RoomId(1));
        world.add_room(Room::new(&mut authored.ecs, RoomId(1), 16.0));
        authored.add_world(world);
        save_game_to_folder(&authored, &resources_dir).unwrap();

        fs::create_dir_all(crate::storage::path_utils::assets_folder().join("sprites")).unwrap();
        fs::write(
            crate::storage::path_utils::assets_folder().join("sprites/player.png"),
            [1_u8, 2, 3, 4],
        )
        .unwrap();

        let mut game = load_game_shell_from_folder(&resources_dir).unwrap();
        assert!(game.current_world().current_room().unwrap().variants.is_empty());
        game.asset_registry
            .register_asset_relative_path(SpriteId(1), "sprites/player.png")
            .unwrap();
        crate::assets::SpriteManager::init_editor_metadata(
            &game.asset_registry,
            &mut game.sprite_manager,
        );

        let lua = Lua::new();
        let mut audio_manager = AudioManager::new::<TestBackend>();
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(1));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Payload(PayloadKey::Room(RoomId(1))));
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(1))));

        let mut driver = HydrationDriver {
            game: &mut game,
            audio_manager: &mut audio_manager,
        };

        let error = driver
            .hydrate_scope(&coordinator, &scope, &CountingFailingLoader::new(), &lua)
            .unwrap_err();
        assert!(matches!(error, HydrationError::TextureLoad(_)));
        assert!(!driver.game.current_world().current_room().unwrap().variants.is_empty());
    }

    #[test]
    fn hydrate_scope_loads_script_assets() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let folder = TestGameFolder::new("hydration_driver_scripts");
        set_game_name(folder.name());
        fs::create_dir_all(crate::storage::path_utils::scripts_folder()).unwrap();
        fs::write(
            crate::storage::path_utils::scripts_folder().join("enemy.lua"),
            "return { update = function() end }",
        )
        .unwrap();

        let lua = Lua::new();
        let mut game = Game::with_name(folder.name());
        let mut audio_manager = AudioManager::new::<TestBackend>();
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(3));
        coordinator.activate_scope(scope.clone());
        let script_id = game
            .script_manager
            .init_script(&mut game.asset_registry, "enemy.lua")
            .unwrap();
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Script(script_id)));

        let mut driver = HydrationDriver {
            game: &mut game,
            audio_manager: &mut audio_manager,
        };

        driver
            .hydrate_scope(&coordinator, &scope, &CountingFailingLoader::new(), &lua)
            .unwrap();
        assert_eq!(driver.game.script_manager.loaded_script_count(), 1);
    }

    #[test]
    fn hydrate_asset_runtime_queues_runtime_sprite_reads() {
        let mut game = Game::default();
        game.asset_registry
            .register_asset_relative_path(SpriteId(8), "sprites/warm.png")
            .unwrap();

        crate::assets::SpriteManager::init_editor_metadata(
            &game.asset_registry,
            &mut game.sprite_manager,
        );
        game.sprite_manager.enable_runtime_texture_loading_for_test();
        game.sprite_manager
            .attach_runtime_file_read_pool_for_test(&FileReadPool::new());
        let mut audio_manager = AudioManager::new::<TestBackend>();

        let mut driver = HydrationDriver {
            game: &mut game,
            audio_manager: &mut audio_manager,
        };

        driver
            .hydrate_asset_runtime(AssetKey::Sprite(SpriteId(8)), &Lua::new())
            .unwrap();

        assert!(driver.game.sprite_manager.has_pending_texture_read(SpriteId(8)));
    }

    #[test]
    fn dehydrate_scope_releases_sound_assets() {
        let mut game = Game::default();
        game.asset_registry
            .register_asset_relative_path(SoundId(1), "music/intro.wav")
            .unwrap();

        let mut audio_manager = AudioManager::new::<TestBackend>();
        audio_manager.claim_sound("music/intro");

        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Boot;
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sound(SoundId(1))));

        let mut driver = HydrationDriver {
            game: &mut game,
            audio_manager: &mut audio_manager,
        };

        driver.dehydrate_scope(&coordinator, &scope);
        let snapshot = driver.audio_manager.diagnostics_snapshot();
        assert_eq!(snapshot.ref_count_entry_count, 0);
    }

    #[test]
    fn dehydrate_asset_decrements_sprite_ref_count() {
        let mut game = Game::default();
        game.asset_registry
            .register_asset_relative_path(SpriteId(1), "sprites/test.png")
            .unwrap();

        crate::assets::SpriteManager::init_editor_metadata(
            &game.asset_registry,
            &mut game.sprite_manager,
        );
        game.sprite_manager.increment_ref(SpriteId(1));

        let mut audio_manager = AudioManager::new::<TestBackend>();
        let mut driver = HydrationDriver {
            game: &mut game,
            audio_manager: &mut audio_manager,
        };

        driver.dehydrate_asset(AssetKey::Sprite(SpriteId(1)));
        assert_eq!(driver.game.sprite_manager.ref_count(&SpriteId(1)), 0);
    }
}
