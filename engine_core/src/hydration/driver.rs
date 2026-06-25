use crate::assets::{AssetKey, AssetRegistry, SpriteManager};
use crate::audio::loader::sound_id_from_asset_path;
use crate::audio::AudioManager;
use crate::hydration::coordinator::HydrationCoordinator;
use crate::hydration::scope::HydrationScope;
use crate::scripting::ScriptManager;
use bishop::prelude::TextureLoader;
use mlua::Lua;

/// Errors surfaced while hydrating claimed assets.
#[derive(Debug)]
pub enum HydrationError {
    UnknownAudioAssetPath(AssetKey),
    TextureLoad(String),
    ScriptLoad(mlua::Error),
}

/// Bridges coordinator asset claims into manager load and release APIs.
pub struct HydrationDriver<'a> {
    pub coordinator: &'a HydrationCoordinator,
    pub asset_registry: &'a AssetRegistry,
    pub sprite_manager: &'a mut SpriteManager,
    pub script_manager: &'a mut ScriptManager,
    pub audio_manager: &'a mut AudioManager,
}

impl<'a> HydrationDriver<'a> {
    /// Hydrates one asset immediately using an explicit texture loader.
    pub fn hydrate_asset(
        &mut self,
        asset: AssetKey,
        texture_loader: &impl TextureLoader,
        lua: &Lua,
    ) -> Result<(), HydrationError> {
        match asset {
            AssetKey::Sprite(id) => self
                .sprite_manager
                .ensure_loaded(texture_loader, id)
                .map_err(HydrationError::TextureLoad),
            AssetKey::Script(id) => self
                .script_manager
                .load_script_table(lua, id)
                .map(|_| ())
                .map_err(HydrationError::ScriptLoad),
            AssetKey::Sound(_) => {
                let Some(path) = self.asset_registry.record(asset).map(|record| &record.path) else {
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

    /// Hydrates one asset for runtime traversal warming.
    pub fn hydrate_asset_runtime(&mut self, asset: AssetKey, lua: &Lua) -> Result<(), HydrationError> {
        match asset {
            AssetKey::Sprite(id) => {
                self.sprite_manager.prewarm_runtime_texture(id);
                Ok(())
            }
            AssetKey::Script(id) => self
                .script_manager
                .load_script_table(lua, id)
                .map(|_| ())
                .map_err(HydrationError::ScriptLoad),
            AssetKey::Sound(_) => {
                let Some(path) = self.asset_registry.record(asset).map(|record| &record.path) else {
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

    /// Releases one hydrated asset.
    pub fn dehydrate_asset(&mut self, asset: AssetKey) {
        match asset {
            AssetKey::Sprite(id) => self.sprite_manager.evict_texture(id),
            AssetKey::Script(id) => self.script_manager.evict_script(id),
            AssetKey::Sound(_) => {
                if let Some(path) = self.asset_registry.record(asset).map(|record| &record.path)
                    && let Some(sound_id) = sound_id_from_asset_path(path)
                {
                    self.audio_manager.release_claimed_sound(&sound_id);
                }
            }
            AssetKey::Prefab(_) | AssetKey::Toml(_) => {}
        }
    }

    /// Hydrates all claimed assets for one scope.
    pub fn hydrate_scope(
        &mut self,
        scope: &HydrationScope,
        texture_loader: &impl TextureLoader,
        lua: &Lua,
    ) -> Result<(), HydrationError> {
        for asset in self.coordinator.claimed_assets(scope) {
            self.hydrate_asset(asset, texture_loader, lua)?;
        }
        Ok(())
    }

    /// Releases all claimed assets for one scope.
    pub fn dehydrate_scope(&mut self, scope: &HydrationScope) {
        for asset in self.coordinator.claimed_assets(scope) {
            self.dehydrate_asset(asset);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{AssetKey, AssetRegistry, SpriteManager};
    use crate::audio::test_utils::{CountingFailingLoader, TestBackend};
    use crate::audio::AudioManager;
    use crate::ecs::{SoundId, SpriteId};
    use crate::engine_global::set_game_name;
    use crate::hydration::Hydratable;
    use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use crate::task::FileReadPool;
    use crate::worlds::RoomId;
    use mlua::Lua;
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
        let mut registry = AssetRegistry::default();
        registry
            .register_asset_relative_path(SpriteId(1), "sprites/player.png")
            .unwrap();
        let mut sprite_manager = SpriteManager::default();
        SpriteManager::init_editor_metadata(&registry, &mut sprite_manager);
        let mut script_manager = crate::scripting::ScriptManager::default();
        let mut audio_manager = AudioManager::new::<TestBackend>();
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(2));
        coordinator.activate_scope(scope.clone());
        coordinator.claim_asset(scope.clone(), AssetKey::Sprite(SpriteId(1)));
        let loader = CountingFailingLoader::new();

        let mut driver = HydrationDriver {
            coordinator: &coordinator,
            asset_registry: &registry,
            sprite_manager: &mut sprite_manager,
            script_manager: &mut script_manager,
            audio_manager: &mut audio_manager,
        };

        let error = driver.hydrate_scope(&scope, &loader, &lua).unwrap_err();
        assert!(matches!(error, HydrationError::TextureLoad(_)));
        assert_eq!(loader.load_calls.get(), 1);
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
        let mut registry = AssetRegistry::default();
        let mut sprite_manager = SpriteManager::default();
        let mut script_manager = crate::scripting::ScriptManager::default();
        let mut audio_manager = AudioManager::new::<TestBackend>();
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(3));
        coordinator.activate_scope(scope.clone());
        let script_id = script_manager.init_script(&mut registry, "enemy.lua").unwrap();
        coordinator.claim_asset(scope.clone(), AssetKey::Script(script_id));

        let mut driver = HydrationDriver {
            coordinator: &coordinator,
            asset_registry: &registry,
            sprite_manager: &mut sprite_manager,
            script_manager: &mut script_manager,
            audio_manager: &mut audio_manager,
        };

        driver
            .hydrate_scope(&scope, &CountingFailingLoader::new(), &lua)
            .unwrap();
        assert_eq!(driver.script_manager.loaded_script_count(), 1);
    }

    #[test]
    fn hydrate_asset_runtime_queues_runtime_sprite_reads() {
        let mut registry = AssetRegistry::default();
        registry
            .register_asset_relative_path(SpriteId(8), "sprites/warm.png")
            .unwrap();

        let mut sprite_manager = SpriteManager::default();
        SpriteManager::init_editor_metadata(&registry, &mut sprite_manager);
        sprite_manager.enable_runtime_texture_loading_for_test();
        sprite_manager.attach_runtime_file_read_pool_for_test(&FileReadPool::new());
        let mut script_manager = crate::scripting::ScriptManager::default();
        let mut audio_manager = AudioManager::new::<TestBackend>();
        let coordinator = HydrationCoordinator::default();

        let mut driver = HydrationDriver {
            coordinator: &coordinator,
            asset_registry: &registry,
            sprite_manager: &mut sprite_manager,
            script_manager: &mut script_manager,
            audio_manager: &mut audio_manager,
        };

        driver
            .hydrate_asset_runtime(AssetKey::Sprite(SpriteId(8)), &Lua::new())
            .unwrap();

        assert!(driver.sprite_manager.has_pending_texture_read(SpriteId(8)));
    }

    #[test]
    fn dehydrate_scope_releases_sound_assets() {
        let mut registry = AssetRegistry::default();
        registry
            .register_asset_relative_path(SoundId(1), "music/intro.wav")
            .unwrap();

        let mut sprite_manager = SpriteManager::default();
        let mut script_manager = crate::scripting::ScriptManager::default();
        let mut audio_manager = AudioManager::new::<TestBackend>();
        audio_manager.claim_sound("music/intro");

        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Boot;
        coordinator.activate_scope(scope.clone());
        coordinator.claim_asset(scope.clone(), AssetKey::Sound(SoundId(1)));

        let mut driver = HydrationDriver {
            coordinator: &coordinator,
            asset_registry: &registry,
            sprite_manager: &mut sprite_manager,
            script_manager: &mut script_manager,
            audio_manager: &mut audio_manager,
        };

        driver.dehydrate_scope(&scope);
        let snapshot = driver.audio_manager.diagnostics_snapshot();
        assert_eq!(snapshot.ref_count_entry_count, 0);
    }

    #[test]
    fn dehydrate_asset_decrements_sprite_ref_count() {
        let mut registry = AssetRegistry::default();
        registry.register_asset_relative_path(SpriteId(1), "sprites/test.png").unwrap();

        let mut sprite_manager = SpriteManager::default();
        SpriteManager::init_editor_metadata(&registry, &mut sprite_manager);
        sprite_manager.increment_ref(SpriteId(1));

        let mut script_manager = crate::scripting::ScriptManager::default();
        let mut audio_manager = AudioManager::new::<TestBackend>();
        let coordinator = HydrationCoordinator::default();

        let mut driver = HydrationDriver {
            coordinator: &coordinator,
            asset_registry: &registry,
            sprite_manager: &mut sprite_manager,
            script_manager: &mut script_manager,
            audio_manager: &mut audio_manager,
        };

        driver.dehydrate_asset(AssetKey::Sprite(SpriteId(1)));
        assert_eq!(driver.sprite_manager.ref_count(&SpriteId(1)), 0);
    }
}
