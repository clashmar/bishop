use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}-{nanos}")
}

#[test]
fn playtest_startup_preview_loads_authored_splash_screens_from_payload_game_name() {
    let game_name = unique_name("PlaytestPreview");
    let game_dir = game_folder(&game_name);
    let resources_dir = game_dir.join(RESOURCES_FOLDER);
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
        name: "{game_name}",
    ),
)
"#
        ),
    )
    .unwrap();

    let startup = load_playtest_startup_preview(&payload_path).unwrap();

    assert_eq!(startup.loading.splash_screens.len(), 1);

    let _ = fs::remove_file(payload_path);
    let _ = fs::remove_dir_all(game_dir);
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
        LoadedStartupData::Playtest { startup_mode, .. } => {
            assert_eq!(startup_mode, StartupMode::Full)
        }
        LoadedStartupData::AgentPayload { startup_mode, .. } => {
            assert_eq!(startup_mode, StartupMode::Full)
        }
        LoadedStartupData::Game { .. } => panic!("expected playtest startup data"),
    }
}

#[test]
fn playtest_skip_mode_bypasses_startup_presentation() {
    let loaded = LoadedStartupData::Playtest {
        startup_asset: StartupAsset::default(),
        room: Room::default(),
        game: Game::default(),
        startup_mode: StartupMode::Skip,
    };

    assert!(loaded.skips_startup_presentation());
}

#[test]
fn agent_payload_skip_mode_bypasses_startup_presentation() {
    let loaded = LoadedStartupData::AgentPayload {
        startup_asset: StartupAsset::default(),
        room: Room::default(),
        game: Game::default(),
        startup_mode: StartupMode::Skip,
    };

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
