use crate::storage::lua_stub_gen::write_menus_lua_from_dir;
use bishop::prelude::*;
use engine_core::menu::{Alignment, LayoutConfig, MenuAction, MenuBackground, MenuBuilder, MenuMode, MenuTemplate, Padding};
use engine_core::storage::{menus_folder, scripts_folder};
use std::fs;
use std::io;
use std::io::Error;

fn default_front_end_menus() -> Vec<MenuTemplate> {
    let start_layout = LayoutConfig::vertical()
        .with_item_size(240.0, 44.0)
        .with_spacing(16.0)
        .with_padding(Padding::uniform(32.0))
        .with_alignment(Alignment::center());

    let start_menu = MenuBuilder::new("start")
        .mode(MenuMode::FrontEnd)
        .background(MenuBackground::SolidColor(Color::new(
            0.05, 0.06, 0.10, 1.0,
        )))
        .layout_group(Rect::new(0.0, 0.0, 1.0, 1.0), start_layout, |group| {
            group
                .label("Title")
                .button("Start", MenuAction::CloseMenu)
                .button("Settings", MenuAction::OpenMenu("settings".to_string()))
        })
        .build();

    let settings_layout = LayoutConfig::vertical()
        .with_item_size(320.0, 44.0)
        .with_spacing(16.0)
        .with_padding(Padding::uniform(32.0))
        .with_alignment(Alignment::center());

    let settings_menu = MenuBuilder::new("settings")
        .mode(MenuMode::FrontEnd)
        .background(MenuBackground::SolidColor(Color::new(
            0.05, 0.06, 0.10, 1.0,
        )))
        .layout_group(Rect::new(0.0, 0.0, 1.0, 1.0), settings_layout, |group| {
            group
                .label("Settings")
                .slider("Master", "master_volume", 0.0, 1.0, 0.05, 1.0)
                .slider("Music", "music_volume", 0.0, 1.0, 0.05, 1.0)
                .slider("SFX", "sfx_volume", 0.0, 1.0, 0.05, 1.0)
                .button("Back", MenuAction::CloseMenu)
        })
        .build();

    vec![start_menu, settings_menu]
}

/// Scaffolds default front-end menus for a new game.
pub fn save_default_front_end_menus() -> io::Result<()> {
    for template in default_front_end_menus() {
        save_menu(&template)?;
    }
    Ok(())
}

/// Saves a menu template to disk.
pub fn save_menu(template: &MenuTemplate) -> io::Result<()> {
    let dir = menus_folder();
    fs::create_dir_all(&dir)?;

    let path = dir.join(format!("{}.ron", template.id));
    let pretty = ron::ser::PrettyConfig::new()
        .separate_tuple_members(true)
        .enumerate_arrays(true);

    let ron = ron::ser::to_string_pretty(template, pretty).map_err(Error::other)?;

    fs::write(path, ron)?;
    write_menus_lua_from_dir(&scripts_folder(), &dir)
}

/// Loads all menu templates from disk.
pub fn load_menus() -> Vec<MenuTemplate> {
    let dir = menus_folder();
    if !dir.exists() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "ron"))
        .filter_map(|entry| {
            let ron = fs::read_to_string(entry.path()).ok()?;
            ron::de::from_str(&ron).ok()
        })
        .collect()
}

/// Deletes a menu template from disk.
pub fn delete_menu(id: &str) -> io::Result<()> {
    let dir = menus_folder();
    let path = dir.join(format!("{}.ron", id));
    if path.exists() {
        fs::remove_file(path)?;
    }
    write_menus_lua_from_dir(&scripts_folder(), &dir)
}
