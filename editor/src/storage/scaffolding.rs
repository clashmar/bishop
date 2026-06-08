use crate::editor_assets::assets::BISHOP_THEME_LUA;
use crate::editor_assets::write_lua_scaffold_configs;
use crate::storage::lua_stub_gen::write_initial_generated_lua_files;
use crate::write_engine_scripts;
use engine_core::constants::paths;
use engine_core::logging::omni_error;
use engine_core::scripting::lua_constants::lua_files;
use engine_core::storage::*;
use std::fs;

pub fn create_game_folders(name: &str) {
    let folders: [(std::path::PathBuf, &str); 9] = [
        (resources_folder_current(), paths::RESOURCES_FOLDER),
        (assets_folder(), paths::ASSETS_FOLDER),
        (scripts_folder(), paths::SCRIPTS_FOLDER),
        (text_folder(), paths::TEXT_FOLDER),
        (prefabs_folder(), paths::PREFABS_FOLDER),
        (themes_folder(), paths::THEMES_FOLDER),
        (windows_folder(), paths::WINDOWS_FOLDER),
        (mac_os_folder(), paths::MAC_OS_FOLDER),
        (editor_metadata_folder(name), paths::EDITOR_METADATA_FOLDER),
    ];

    for (path, folder) in folders {
        if let Err(e) = fs::create_dir_all(&path) {
            omni_error!("Could not create {folder} folder '{}': {e}", path.display());
        }
    }

    if let Err(e) = write_engine_scripts(&scripts_folder()) {
        omni_error!("Could not write _engine scripts: {e}");
    }

    if let Err(e) = write_lua_scaffold_configs(&game_folder(name)) {
        omni_error!("Could not write Lua scaffold configs: {e}");
    }

    if let Err(e) = write_initial_generated_lua_files(&scripts_folder()) {
        omni_error!("Could not write initial generated Lua files: {e}");
    }

    let theme_path = themes_folder().join(lua_files::BISHOP_THEME);
    if !theme_path.exists() {
        if let Err(e) = fs::write(&theme_path, BISHOP_THEME_LUA) {
            omni_error!("Could not write sample theme: {e}");
        }
    }

    let main_lua = scripts_folder().join("main.lua");
    if !main_lua.exists() {
        if let Err(e) = fs::write(&main_lua, "") {
            omni_error!("Could not create main.lua: {e}");
        }
    }

    for path in [sfx_folder(), music_folder()] {
        if let Err(e) = fs::create_dir_all(&path) {
            omni_error!("Could not create audio folder '{}': {e}", path.display());
        }
    }

    create_default_text_files();
}

/// Creates the default text manifest and language folders.
fn create_default_text_files() {
    let text_root = text_folder();

    let manifest_path = text_root.join(paths::LANGUAGE_MANIFEST);
    if !manifest_path.exists() {
        let manifest_content = r#"# Text manifest configuration
default_language = "en"
"#;
        if let Err(e) = fs::write(&manifest_path, manifest_content) {
            omni_error!("Could not create text manifest: {e}");
        }
    }

    let en_dialogue = text_root
        .join(paths::TEXT_LANGUAGE_FOLDER)
        .join(paths::DIALOGUE_FOLDER);
    if let Err(e) = fs::create_dir_all(&en_dialogue) {
        omni_error!(
            "Could not create {}/{}/{} folder: {e}",
            paths::TEXT_FOLDER,
            paths::TEXT_LANGUAGE_FOLDER,
            paths::DIALOGUE_FOLDER
        );
    }

    let en_ui = text_root
        .join(paths::TEXT_LANGUAGE_FOLDER)
        .join(paths::UI_TEXT_FOLDER);
    if let Err(e) = fs::create_dir_all(&en_ui) {
        omni_error!(
            "Could not create {}/{}/{} folder: {e}",
            paths::TEXT_FOLDER,
            paths::TEXT_LANGUAGE_FOLDER,
            paths::UI_TEXT_FOLDER
        );
    }

    let start_ui_path = en_ui.join("start.toml");
    if !start_ui_path.exists() {
        let content = r#"Title = "NEW GAME"
Start = "Start"
Settings = "Settings"
"#;
        if let Err(e) = fs::write(&start_ui_path, content) {
            omni_error!(
                "Could not create {}/{}/start.toml: {e}",
                paths::TEXT_LANGUAGE_FOLDER,
                paths::UI_TEXT_FOLDER
            );
        }
    }

    let settings_ui_path = en_ui.join("settings.toml");
    if !settings_ui_path.exists() {
        let content = r#"Settings = "Settings"
Master = "Master Volume"
Music = "Music Volume"
SFX = "SFX Volume"
Back = "Back"
"#;
        if let Err(e) = fs::write(&settings_ui_path, content) {
            omni_error!(
                "Could not create {}/{}/settings.toml: {e}",
                paths::TEXT_LANGUAGE_FOLDER,
                paths::UI_TEXT_FOLDER
            );
        }
    }
}
