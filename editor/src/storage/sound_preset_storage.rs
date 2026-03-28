use engine_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};

pub const SOUND_PRESETS_RON: &str = "sound_presets.ron";

/// Project-wide sound preset library persisted by the editor.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundPresetLibrary {
    /// Presets keyed by their display name.
    pub presets: std::collections::HashMap<String, AudioGroup>,
}

thread_local! {
    static CURRENT_SOUND_PRESET_LIBRARY: RefCell<SoundPresetLibrary> =
        RefCell::new(SoundPresetLibrary::default());
}

/// Replaces the current in-memory sound preset library for the active project.
pub fn set_current_sound_preset_library(library: SoundPresetLibrary) {
    CURRENT_SOUND_PRESET_LIBRARY.with(|current| {
        *current.borrow_mut() = library;
    });
}

/// Returns a clone of the current in-memory sound preset library.
pub fn current_sound_preset_library() -> SoundPresetLibrary {
    CURRENT_SOUND_PRESET_LIBRARY.with(|current| current.borrow().clone())
}

/// Mutably borrows the current in-memory sound preset library.
pub fn with_sound_preset_library_mut<R>(f: impl FnOnce(&mut SoundPresetLibrary) -> R) -> R {
    CURRENT_SOUND_PRESET_LIBRARY.with(|current| f(&mut current.borrow_mut()))
}

/// Removes a preset from the current in-memory sound preset library.
pub fn delete_sound_preset(preset_name: &str) -> bool {
    with_sound_preset_library_mut(|library| library.presets.remove(preset_name).is_some())
}

/// Loads the project's sound preset library from disk.
pub fn load_sound_preset_library(game_name: &str) -> io::Result<SoundPresetLibrary> {
    let path = game_folder(game_name).join(SOUND_PRESETS_RON);

    match fs::read_to_string(path) {
        Ok(ron) => ron::from_str(&ron).map_err(|error| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Could not parse sound preset library: {error}"),
            )
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(SoundPresetLibrary::default()),
        Err(error) => Err(error),
    }
}

/// Saves the project's sound preset library to disk.
pub fn save_sound_preset_library(game_name: &str, library: &SoundPresetLibrary) -> io::Result<()> {
    let path = game_folder(game_name).join(SOUND_PRESETS_RON);
    fs::create_dir_all(game_folder(game_name))?;

    let ron =
        ron::ser::to_string_pretty(library, ron::ser::PrettyConfig::new()).map_err(Error::other)?;

    fs::write(path, ron)?;
    set_current_sound_preset_library(library.clone());
    Ok(())
}

/// Collects all sound group names from the ECS and preset library.
pub fn collect_sound_group_names(ecs: &Ecs, library: &SoundPresetLibrary) -> Vec<String> {
    let mut names = BTreeSet::new();

    names.extend(library.presets.keys().cloned());

    for source in ecs.get_store::<AudioSource>().data.values() {
        for group_id in source.groups.keys() {
            if let SoundGroupId::Custom(name) = group_id {
                names.insert(name.clone());
            }
        }
    }

    names.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::editor_storage::save_game;
    use engine_core::audio::AudioGroup;
    use engine_core::scripting::lua_constants::{ENGINE_DIR, SOUNDS_FILE};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct TestGameFolder {
        name: String,
    }

    impl TestGameFolder {
        fn new(prefix: &str) -> Self {
            let name = format!("{prefix}_{}", Uuid::new_v4());
            let path = game_folder(&name);
            let _ = fs::remove_dir_all(&path);
            Self { name }
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn path(&self) -> PathBuf {
            game_folder(&self.name)
        }
    }

    impl Drop for TestGameFolder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.path());
        }
    }

    #[test]
    fn load_sound_preset_library_defaults_when_file_is_missing() {
        let test_game = TestGameFolder::new("sound_presets_missing");

        let library = load_sound_preset_library(test_game.name()).unwrap();

        assert_eq!(library, SoundPresetLibrary::default());
    }

    #[test]
    fn load_sound_preset_library_returns_error_for_invalid_data() {
        let test_game = TestGameFolder::new("sound_presets_invalid");
        fs::create_dir_all(test_game.path()).unwrap();
        fs::write(
            test_game.path().join(SOUND_PRESETS_RON),
            "this is not valid ron",
        )
        .unwrap();

        let error = load_sound_preset_library(test_game.name()).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn delete_sound_preset_removes_entry_from_current_library() {
        set_current_sound_preset_library(SoundPresetLibrary {
            presets: HashMap::from([("Ambient".to_string(), AudioGroup::default())]),
        });

        assert!(delete_sound_preset("Ambient"));

        assert!(!current_sound_preset_library()
            .presets
            .contains_key("Ambient"));
    }

    #[test]
    fn save_game_returns_error_when_sounds_lua_cannot_be_written() {
        let test_game = TestGameFolder::new("save_game_sounds_lua_error");
        set_game_name(test_game.name());

        let sounds_lua_path = scripts_folder().join(ENGINE_DIR).join(SOUNDS_FILE);
        fs::create_dir_all(&sounds_lua_path).unwrap();

        let game = Game {
            name: test_game.name().to_string(),
            ..Default::default()
        };

        let error = save_game(&game).unwrap_err();

        assert!(
            matches!(
                error.kind(),
                ErrorKind::IsADirectory | ErrorKind::PermissionDenied | ErrorKind::Other
            ),
            "unexpected error kind: {:?}",
            error.kind()
        );
        assert!(!resources_folder_current().join(GAME_RON).exists());
    }

    #[test]
    fn collect_sound_group_names_merges_presets_and_local_groups() {
        let mut ecs = Ecs::default();
        let entity = ecs.create_entity().finish();

        let mut source = AudioSource::default();
        source.groups.insert(
            SoundGroupId::Custom("Talk".to_string()),
            AudioGroup::default(),
        );
        source.groups.insert(
            SoundGroupId::Custom("Footsteps".to_string()),
            AudioGroup::default(),
        );
        ecs.add_component_to_entity(entity, source);

        let library = SoundPresetLibrary {
            presets: HashMap::from([("Ambient".to_string(), AudioGroup::default())]),
        };

        let names = collect_sound_group_names(&ecs, &library);

        assert_eq!(
            names,
            vec![
                "Ambient".to_string(),
                "Footsteps".to_string(),
                "Talk".to_string(),
            ]
        );
    }
}
