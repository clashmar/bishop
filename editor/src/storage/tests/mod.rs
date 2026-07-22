use super::game_io::{
    create_new_game, list_game_names, load_game_by_name, most_recent_game_name, save_game,
};
use super::prefab_palettes::{load_prefab_palette_state, save_prefab_palette_state};
use crate::prefab::palette::PrefabPaletteState;
use crate::storage::lua_stub_gen::write_prefabs_lua;
use crate::storage::scaffolding::create_game_folders;
use engine_core::assets::*;
use engine_core::constants::{extensions, paths};
use engine_core::ecs::*;
use engine_core::engine_global::set_game_name;
use engine_core::game::Game;
use engine_core::scripting::event_tags::event_tag::EventTag;
use engine_core::scripting::lua_constants::{lua_dirs, lua_files};
use engine_core::scripting::ScriptManager;
use engine_core::storage::path_utils::sanitise_name;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use engine_core::storage::*;
use engine_core::tiles::{TileComponent, TileDef};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

mod asset_registry_tests;
mod game_io_tests;
mod prefab_palette_tests;
mod prefab_storage_tests;
mod scaffolding_tests;
