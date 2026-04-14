use super::*;
use engine_core::agents::payload::AgentPayloadSpec;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}-{nanos}")
}

fn payload_startup_ron() -> String {
    ron::ser::to_string_pretty(&StartupAsset::default(), ron::ser::PrettyConfig::new()).unwrap()
}

fn editor_payload_ron(game_name: &str, room: &Room, startup_mode: StartupMode) -> String {
    #[derive(serde::Serialize)]
    struct Payload<'a> {
        room: &'a Room,
        game: &'a Game,
        startup_ron: Option<String>,
        startup_mode: StartupMode,
    }

    let game = Game {
        name: game_name.to_string(),
        ..Game::default()
    };

    ron::ser::to_string_pretty(
        &Payload {
            room,
            game: &game,
            startup_ron: Some(payload_startup_ron()),
            startup_mode,
        },
        ron::ser::PrettyConfig::new(),
    )
    .unwrap()
}

fn seeded_payload_path(game_name: &str) -> std::path::PathBuf {
    let payload = AgentPayloadSpec::synthetic(game_name)
        .add_room("Seeded Room")
        .build()
        .unwrap();
    let path = std::env::temp_dir().join(format!("{}.ron", unique_name("seeded-payload")));
    fs::write(
        &path,
        ron::ser::to_string_pretty(&payload, ron::ser::PrettyConfig::new()).unwrap(),
    )
    .unwrap();
    path
}

#[test]
fn playtest_startup_preview_loads_authored_splash_screens_from_payload_game_name() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("PlaytestPreview");
    let resources_dir = test_game.path().join(RESOURCES_FOLDER);
    fs::create_dir_all(&resources_dir).unwrap();
    let startup = StartupAsset {
        loading: super::super::LoadingConfig {
            splash_screens: vec![StartupScreenSpec {
                min_duration_secs: 1.0,
                background_color: [0.1, 0.2, 0.3, 1.0],
                content: StartupScreenContent::Text {
                    text: "Splash".to_string(),
                    font_size: 42.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                },
            }],
            fallback_screen: StartupScreenSpec::default(),
        },
        start_menu_id: "start".to_string(),
    };
    fs::write(
        resources_dir.join("startup.ron"),
        ron::ser::to_string_pretty(&startup, ron::ser::PrettyConfig::new()).unwrap(),
    )
    .unwrap();

    let payload_path = std::env::temp_dir().join(format!("{}.ron", unique_name("payload")));
    fs::write(
        &payload_path,
        format!(
            r#"
(
    game: (
        name: "{}",
    ),
)
"#,
            test_game.name(),
        ),
    )
    .unwrap();

    let startup = load_playtest_startup_preview(&payload_path).unwrap();

    assert_eq!(startup.loading.splash_screens.len(), 1);

    let _ = fs::remove_file(payload_path);
}

#[test]
fn playtest_startup_preview_prefers_embedded_payload() {
    let game_name = unique_name("EmbeddedPlaytestPreview");
    let payload_path = std::env::temp_dir().join(format!("{}.ron", unique_name("payload")));
    let embedded_startup = StartupAsset {
        loading: super::super::LoadingConfig {
            splash_screens: vec![StartupScreenSpec {
                min_duration_secs: 3.0,
                background_color: [0.4, 0.3, 0.2, 1.0],
                content: StartupScreenContent::Text {
                    text: "Embedded".to_string(),
                    font_size: 30.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                },
            }],
            fallback_screen: StartupScreenSpec::default(),
        },
        start_menu_id: "start".to_string(),
    };

    #[derive(serde::Serialize)]
    struct Payload<'a> {
        game: PayloadGame<'a>,
        startup_ron: Option<String>,
    }

    #[derive(serde::Serialize)]
    struct PayloadGame<'a> {
        name: &'a str,
    }

    fs::write(
        &payload_path,
        ron::ser::to_string_pretty(
            &Payload {
                game: PayloadGame { name: &game_name },
                startup_ron: Some(
                    ron::ser::to_string_pretty(&embedded_startup, ron::ser::PrettyConfig::new())
                        .unwrap(),
                ),
            },
            ron::ser::PrettyConfig::new(),
        )
        .unwrap(),
    )
    .unwrap();

    let startup = load_playtest_startup_preview(&payload_path).unwrap();

    assert_eq!(startup.loading.splash_screens.len(), 1);
    match &startup.loading.splash_screens[0].content {
        StartupScreenContent::Text { text, .. } => assert_eq!(text, "Embedded"),
    }

    let _ = fs::remove_file(payload_path);
}

#[test]
fn splash_duration_starts_from_first_visible_frame() {
    assert!(!splash_min_duration_elapsed(10.0, 10.0, 1.5));
    assert!(!splash_min_duration_elapsed(10.0, 11.49, 1.5));
    assert!(splash_min_duration_elapsed(10.0, 11.5, 1.5));
}

#[test]
fn fallback_min_duration_starts_from_first_visible_frame() {
    assert!(!splash_min_duration_elapsed(20.0, 20.0, 5.0));
    assert!(!splash_min_duration_elapsed(20.0, 24.99, 5.0));
    assert!(splash_min_duration_elapsed(20.0, 25.0, 5.0));
}

#[test]
fn parse_startup_data_uses_startup_mode_from_payload() {
    let payload = r#"
(
    room: (
        name: "Room",
    ),
    game: (
        name: "Demo",
    ),
    startup_mode: Full,
)
"#;

    let loaded = parse_startup_data(LoadedStartupFiles::Playtest {
        payload_ron: payload.to_string(),
    })
    .unwrap();

    match loaded {
        LoadedStartupData::Playtest(LoadedPlaytestData { startup_mode, .. }) => {
            assert_eq!(startup_mode, StartupMode::Full)
        }
        LoadedStartupData::Game { .. } => panic!("expected playtest startup data"),
    }
}

#[test]
fn payload_backed_startup_data_builds_same_room_game_shape_for_editor_and_seeded_sources() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("PayloadBackedStartupData");
    let room = Room {
        id: RoomId(1),
        name: "Seeded Room".to_string(),
        ..Room::default()
    };
    let resources_dir = test_game.path().join(RESOURCES_FOLDER);
    fs::create_dir_all(&resources_dir).unwrap();
    fs::write(resources_dir.join("startup.ron"), payload_startup_ron()).unwrap();

    let editor_loaded = parse_startup_data(LoadedStartupFiles::Playtest {
        payload_ron: editor_payload_ron(test_game.name(), &room, StartupMode::Skip),
    })
    .unwrap();
    let seeded_path = seeded_payload_path(test_game.name());
    let seeded_loaded = parse_startup_data(LoadedStartupFiles::AgentPayload {
        payload_path: seeded_path.display().to_string(),
    })
    .unwrap();

    let editor_playtest = match editor_loaded {
        LoadedStartupData::Playtest(LoadedPlaytestData {
            room,
            game,
            startup_mode,
            ..
        }) => Some((room.id, game.name, startup_mode)),
        LoadedStartupData::Game { .. } => None,
    };
    let seeded_playtest = match seeded_loaded {
        LoadedStartupData::Playtest(LoadedPlaytestData {
            room,
            game,
            startup_mode,
            ..
        }) => Some((room.id, game.name, startup_mode)),
        LoadedStartupData::Game { .. } => None,
    };

    assert_eq!(editor_playtest, seeded_playtest);

    let _ = fs::remove_file(seeded_path);
}

#[test]
fn playtest_skip_mode_bypasses_startup_presentation() {
    let loaded = LoadedStartupData::Playtest(LoadedPlaytestData {
        startup_asset: StartupAsset::default(),
        room: Room::default(),
        game: Game::default(),
        startup_mode: StartupMode::Skip,
    });

    assert!(loaded.skips_startup_presentation());
}

#[test]
fn playtest_game_name_reads_name_from_payload() {
    let payload = r#"
(
    room: (
        name: "Room",
    ),
    game: (
        name: "Demo",
    ),
)
"#;

    let game_name = parse_playtest_game_name(payload).unwrap();

    assert_eq!(game_name, "Demo");
}
