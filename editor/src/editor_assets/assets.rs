// editor/src/editor_assets/editor_assets.rs
#![allow(unused)]
use std::path::{Path, PathBuf};
use engine_core::prelude::*;
use std::sync::{LazyLock, OnceLock};
use std::{env, fs, io};
use bishop::prelude::*;

/// Windows .exe for the game binary.
pub static GAME_EXE: &[u8] = include_bytes!(
    concat!(env!("CARGO_MANIFEST_DIR"), "/binaries/game.exe")
);

/// Windows .exe for the game playtest binary.
pub static PLAYTEST_EXE: &[u8] = include_bytes!(
    concat!(env!("CARGO_MANIFEST_DIR"), "/binaries/game-playtest.exe")
);

/// Mac binary for the game.
pub static GAME_BIN: &[u8] = include_bytes!(
    concat!(env!("CARGO_MANIFEST_DIR"), "/binaries/game")
);

/// Mac binary for the game. playtest
pub static PLAYTEST_BIN: &[u8] = include_bytes!(
    concat!(env!("CARGO_MANIFEST_DIR"), "/binaries/game-playtest")
);

pub static ICON_SMALL: LazyLock<[u8; 16 * 16 * 4]> = LazyLock::new(|| {
    load_rgba_resized::<{ 16 * 16 * 4 }>(include_bytes!("icon.png"), 16)
});

pub static ICON_MEDIUM: LazyLock<[u8; 32 * 32 * 4]> = LazyLock::new(|| {
    load_rgba_resized::<{ 32 * 32 * 4 }>(include_bytes!("icon.png"), 32)
});

pub static ICON_BIG: LazyLock<[u8; 64 * 64 * 4]> = LazyLock::new(|| {
    load_rgba_resized::<{ 64 * 64 * 4 }>(include_bytes!("icon.png"), 64)
});

static SELECT_ICON: OnceLock<Texture2D> = OnceLock::new();
static EDIT_ICON: OnceLock<Texture2D> = OnceLock::new();
static CREATE_ICON: OnceLock<Texture2D> = OnceLock::new();
static DELETE_ICON: OnceLock<Texture2D> = OnceLock::new();
static MOVE_ICON: OnceLock<Texture2D> = OnceLock::new();
static TILE_ICON: OnceLock<Texture2D> = OnceLock::new();
static ENTITY_ICON: OnceLock<Texture2D> = OnceLock::new();
static GRID_ICON: OnceLock<Texture2D> = OnceLock::new();
static EXIT_ICON: OnceLock<Texture2D> = OnceLock::new();
static CIRCLE_120PX: OnceLock<Texture2D> = OnceLock::new();

/// Loads all editor icon textures. Must be called once after the graphics context is ready.
pub fn init_editor_icons(loader: &impl TextureLoader) {
    let load = |data: &[u8]| loader.load_texture_from_bytes(data).unwrap_or_else(|_| loader.empty_texture());
    let _ = SELECT_ICON.set(load(include_bytes!("icons/select.png")));
    let _ = EDIT_ICON.set(load(include_bytes!("icons/edit.png")));
    let _ = CREATE_ICON.set(load(include_bytes!("icons/create.png")));
    let _ = DELETE_ICON.set(load(include_bytes!("icons/delete.png")));
    let _ = MOVE_ICON.set(load(include_bytes!("icons/move.png")));
    let _ = TILE_ICON.set(load(include_bytes!("icons/tile.png")));
    let _ = ENTITY_ICON.set(load(include_bytes!("icons/entity.png")));
    let _ = GRID_ICON.set(load(include_bytes!("icons/grid.png")));
    let _ = EXIT_ICON.set(load(include_bytes!("icons/exit.png")));
    let _ = CIRCLE_120PX.set(load(include_bytes!("textures/circle120px.png")));
}

pub fn select_icon() -> &'static Texture2D { SELECT_ICON.get().expect("Editor icons not initialized") }
pub fn edit_icon() -> &'static Texture2D { EDIT_ICON.get().expect("Editor icons not initialized") }
pub fn create_icon() -> &'static Texture2D { CREATE_ICON.get().expect("Editor icons not initialized") }
pub fn delete_icon() -> &'static Texture2D { DELETE_ICON.get().expect("Editor icons not initialized") }
pub fn move_icon() -> &'static Texture2D { MOVE_ICON.get().expect("Editor icons not initialized") }
pub fn tile_icon() -> &'static Texture2D { TILE_ICON.get().expect("Editor icons not initialized") }
pub fn entity_icon() -> &'static Texture2D { ENTITY_ICON.get().expect("Editor icons not initialized") }
pub fn grid_icon() -> &'static Texture2D { GRID_ICON.get().expect("Editor icons not initialized") }
pub fn exit_icon() -> &'static Texture2D { EXIT_ICON.get().expect("Editor icons not initialized") }
pub fn circle_120px() -> &'static Texture2D { CIRCLE_120PX.get().expect("Editor icons not initialized") }

// Include the auto-generated ENGINE_SCRIPTS array from build.rs
include!("engine_scripts.rs");

pub use crate::editor_assets::sounds_lua::generate_sounds_lua;

/// Write embedded _engine scripts to the specified scripts folder.
pub fn write_engine_scripts(scripts_folder: &Path) -> io::Result<()> {
    let engine_folder = scripts_folder.join("_engine");
    fs::create_dir_all(&engine_folder)?;

    for (filename, content) in ENGINE_SCRIPTS {
        fs::write(engine_folder.join(filename), content)?;
    }

    hide_folder(&engine_folder);
    Ok(())
}

/// Hide a folder using platform-specific methods.
fn hide_folder(path: &Path) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("attrib")
            .args(["+h", &path.to_string_lossy()])
            .output();
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("chflags")
            .args(["hidden", &path.to_string_lossy()])
            .output();
    }
}

/// Writes animations.lua with both built-in and custom clips.
pub fn write_animations_lua(scripts_folder: &Path, custom_clips: &[String]) -> io::Result<()> {
    let engine_folder = scripts_folder.join("_engine");
    fs::create_dir_all(&engine_folder)?;
    fs::write(engine_folder.join("animations.lua"), generate_animations_lua(custom_clips))
}

/// Writes sounds.lua with the supplied group names.
pub fn write_sounds_lua(scripts_folder: &Path, group_names: &[String]) -> io::Result<()> {
    let engine_folder = scripts_folder.join("_engine");
    fs::create_dir_all(&engine_folder)?;
    fs::write(engine_folder.join("sounds.lua"), generate_sounds_lua(group_names))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::editor_storage::{AudioGroupPreset, SoundPresetLibrary};
    use engine_core::audio::{AudioGroup, AudioSource, SoundGroupId};
    use std::collections::HashMap;

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
            presets: HashMap::from([(
                "Ambient".to_string(),
                AudioGroupPreset::default(),
            )]),
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
