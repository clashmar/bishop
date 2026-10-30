use crate::scripting::script_system::ScriptSystem;
use crate::startup::browser::place_browser_player_at_start;
use crate::startup::prepare_browser_game;
use bishop::prelude::{Texture2D, TextureLoader, Vec2};
use engine_core::constants::paths;
use engine_core::ecs::{Active, CurrentRoom, Player, Script, ScriptId, Transform, WorldEntry};
use engine_core::game::Game;
use engine_core::prefab::{PrefabAsset, PrefabId, PrefabNode};
use engine_core::scripting::{EventBus, install_prefetched};
use engine_core::scripting::lua_constants::{lua_dirs, lua_files};
use engine_core::storage::{
    ResourceEntry, ResourceManifest, ResourceSource, load_game_shell_from_source,
    room_payload_path, save_game_to_folder, world_descriptor_path,
};
use engine_core::worlds::{Room, RoomId, RoomLayer, World, WorldId};
use mlua::Lua;
use std::collections::HashMap;
use crate::engine::GameInstance;
use std::fs;
use std::path::{Path, PathBuf};

fn prefetched_scripts(entries: &[(&Path, &[u8])]) -> ResourceSource {
    let mut files = HashMap::new();
    let mut manifest = Vec::new();
    for (path, bytes) in entries {
        let key = path.components().map(|component| component.as_os_str().to_str().unwrap())
            .collect::<Vec<_>>().join("/");
        files.insert(key.clone(), bytes.to_vec());
        manifest.push(ResourceEntry { path: key, byte_len: bytes.len() });
    }
    manifest.sort_by(|left, right| left.path.cmp(&right.path));
    ResourceSource::from_prefetched(ResourceManifest { files: manifest }, files).unwrap()
}

#[test]
fn browser_script_when_main_is_missing_reports_path_without_silent_success() {
    let lua = Lua::new();
    let source = prefetched_scripts(&[]);
    install_prefetched(&lua, source.prefetched_lua_sources().unwrap()).unwrap();
    let main = Path::new(paths::SCRIPTS_FOLDER).join(lua_files::MAIN);

    let error = ScriptSystem::init_from_source(&lua, &EventBus::default(), &source).unwrap_err();

    assert!(error.to_string().contains(&main.display().to_string()), "{error}");
}

#[test]
fn browser_script_when_prefetched_runs_globals_main_and_theme_module() {
    let main = Path::new(paths::SCRIPTS_FOLDER).join(lua_files::MAIN);
    let globals = Path::new(paths::SCRIPTS_FOLDER).join(lua_dirs::ENGINE).join(lua_files::GLOBALS);
    let helper = Path::new(paths::THEMES_FOLDER).join("helper.lua");
    let source = prefetched_scripts(&[
        (&main, b"browser_loaded = browser_globals .. require('helper').kind"),
        (&globals, b"browser_globals = 'prefetched-'"),
        (&helper, b"return { kind = 'theme' }"),
    ]);
    let lua = Lua::new();
    install_prefetched(&lua, source.prefetched_lua_sources().unwrap()).unwrap();

    ScriptSystem::init_from_source(&lua, &EventBus::default(), &source).unwrap();

    assert_eq!(lua.globals().get::<String>("browser_loaded").unwrap(), "prefetched-theme");
}

struct FailingTextureLoader;

impl TextureLoader for FailingTextureLoader {
    fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
        Err("synthetic GPU unavailable".into())
    }

    fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
        Err("synthetic filesystem unavailable".into())
    }

    fn empty_texture(&self) -> Texture2D {
        panic!("synthetic no-PNG startup must not allocate a GPU texture")
    }
}

fn synthetic_game_source(
    main: Option<&[u8]>,
    entity_script: Option<&[u8]>,
    prefab: Option<Option<&[u8]>>,
) -> (Game, ResourceSource) {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path();
    let mut authored = Game::with_name("browser_fixture");
    let (world_id, room_id) = (WorldId(7), RoomId(9));
    let mut world = World::new(world_id, "synthetic".into(), 16.0);
    world.current_room_id = Some(room_id);
    world.add_room(Room::new(&mut authored.ecs, room_id, 16.0));
    authored.add_world(world);
    authored.current_world_id = Some(world_id);
    if prefab.is_some() {
        authored.asset_registry.register_asset_relative_path(PrefabId(1), "Fixture.prefab").unwrap();
    }
    if entity_script.is_some() {
        authored.asset_registry.register_asset_relative_path(ScriptId(3), "entity.lua").unwrap();
        authored.ecs.create_entity().with(Script {
            script_id: ScriptId(3), ..Default::default()
        }).with_current_room(room_id).finish();
    }
    save_game_to_folder(&authored, root).unwrap();

    let paths = [
        root.join(paths::GAME_RON), root.join(paths::ECS_RON),
        root.join(paths::ASSET_REGISTRY_RON), root.join(paths::TILE_REGISTRY_RON),
        world_descriptor_path(root, world_id), room_payload_path(root, room_id),
    ];
    let mut files: Vec<(PathBuf, Vec<u8>)> = paths.into_iter().map(|path| {
        let bytes = fs::read(&path).unwrap();
        (path.strip_prefix(root).unwrap().to_path_buf(), bytes)
    }).collect();
    if let Some(bytes) = main {
        files.push((Path::new(paths::SCRIPTS_FOLDER).join(lua_files::MAIN), bytes.to_vec()));
    }
    if let Some(bytes) = entity_script {
        files.push((Path::new(paths::SCRIPTS_FOLDER).join("entity.lua"), bytes.to_vec()));
    }
    if let Some(Some(bytes)) = prefab {
        files.push((Path::new(paths::PREFABS_FOLDER).join("Fixture.prefab"), bytes.to_vec()));
    }
    let manifest = ResourceManifest { files: files.iter().map(|(path, bytes)| ResourceEntry {
        path: path.components().map(|part| part.as_os_str().to_str().unwrap()).collect::<Vec<_>>().join("/"),
        byte_len: bytes.len(),
    }).collect() };
    let mut manifest = manifest;
    manifest.files.sort_by(|left, right| left.path.cmp(&right.path));
    let contents = files.into_iter().map(|(path, bytes)| {
        (path.components().map(|part| part.as_os_str().to_str().unwrap()).collect::<Vec<_>>().join("/"), bytes)
    }).collect();
    let source = ResourceSource::from_prefetched(manifest, contents).unwrap();
    (load_game_shell_from_source(&source).unwrap(), source)
}

#[test]
fn browser_game_when_main_is_missing_reports_startup_failure() {
    let (game, source) = synthetic_game_source(None, None, None);
    let error = prepare_browser_game(game, source, &FailingTextureLoader).err().unwrap();
    let main = Path::new(paths::SCRIPTS_FOLDER).join(lua_files::MAIN);
    assert!(error.contains(&main.display().to_string()), "{error}");
}

#[test]
fn browser_game_when_authored_scripts_are_present_prepares_real_instance() {
    let (game, source) = synthetic_game_source(
        Some(b"browser_boot = 'main'"),
        Some(b"return { init = function(self) end, update = function(self, dt) end }"),
        None,
    );
    let prepared = prepare_browser_game(game, source, &FailingTextureLoader).unwrap();

    assert_eq!(prepared.builder.lua.globals().get::<String>("browser_boot").unwrap(), "main");
    assert_eq!(prepared.prepared.game.script_manager.instance_count(), 1);
    assert_eq!(prepared.prepared.game.current_world().current_room_id, Some(RoomId(9)));
}

fn fixture_prefab_bytes() -> Vec<u8> {
    ron::to_string(&PrefabAsset {
        id: PrefabId(1),
        name: "Fixture".into(),
        next_node_id: 2,
        root_node_id: 1,
        nodes: vec![PrefabNode {
            node_id: 1, parent_node_id: None, components: Vec::new(),
        }],
    }).unwrap().into_bytes()
}

#[test]
fn browser_game_when_prefab_is_prefetched_loads_the_runtime_definition() {
    let bytes = fixture_prefab_bytes();
    let (game, source) = synthetic_game_source(Some(b"browser_boot = 'main'"), None, Some(Some(&bytes)));

    let prepared = prepare_browser_game(game, source, &FailingTextureLoader).unwrap();

    assert!(prepared.prepared.game.prefab_manager.prefab_named("Fixture").is_some());
}

#[test]
fn browser_start_when_player_is_unroomed_places_them_at_authored_start_entry() {
    let mut game = Game::with_name("start_entry_fixture");
    let mut world = World::new(WorldId(7), "synthetic".into(), 16.0);
    world.add_room(Room::new(&mut game.ecs, RoomId(9), 16.0));
    world.current_room_id = Some(RoomId(9));
    game.add_world(world);
    let expected = Vec2::new(18.0, 25.0);
    game.ecs.create_entity().with(WorldEntry {
        name: WorldEntry::START_NAME.to_string(), is_start: true,
    }).with(Transform { position: expected, ..Default::default() })
        .with_current_room_layer(RoomId(9), RoomLayer::Back).finish();
    let player = game.ecs.create_entity().with(Player).with(Active::default())
        .with(Transform::default()).finish();
    let mut instance = GameInstance {
        game, prev_positions: HashMap::new(), traversal_residency_diagnostics: None,
    };

    place_browser_player_at_start(&Lua::new(), &mut instance).unwrap();

    assert_eq!(instance.game.ecs.get::<CurrentRoom>(player).copied(),
        Some(CurrentRoom::new(RoomId(9), RoomLayer::Back)));
    assert_eq!(instance.game.ecs.get::<Transform>(player).unwrap().position, expected);
}

#[test]
fn browser_game_when_registered_prefab_is_missing_reports_its_path() {
    let (game, source) = synthetic_game_source(Some(b"browser_boot = 'main'"), None, Some(None));

    let error = prepare_browser_game(game, source, &FailingTextureLoader).err().unwrap();

    let expected = Path::new(paths::PREFABS_FOLDER).join("Fixture.prefab");
    assert!(error.contains(&expected.display().to_string()), "{error}");
}
