use std::collections::HashMap;

use bishop::input::{KeyCode, KeyboardMouseState};
use engine_core::game::GameDataManifest;
use mlua::{Function, Lua, Table};
use web_shell::{FIXTURE_ENTRY_MODULE, FIXTURE_MANIFEST_PATH};

pub(crate) fn fixture_axis(input: &KeyboardMouseState) -> (f32, f32) {
    let left = input.is_key_down(KeyCode::A) || input.is_key_down(KeyCode::Left);
    let right = input.is_key_down(KeyCode::D) || input.is_key_down(KeyCode::Right);
    let up = input.is_key_down(KeyCode::W) || input.is_key_down(KeyCode::Up);
    let down = input.is_key_down(KeyCode::S) || input.is_key_down(KeyCode::Down);
    (
        f32::from(right as u8) - f32::from(left as u8),
        f32::from(down as u8) - f32::from(up as u8),
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FixtureState {
    pub x: f32,
    pub y: f32,
    pub goal_x: f32,
    pub goal_y: f32,
    pub won: bool,
}

/// Scripted state for the dedicated browser fixture.
pub(crate) struct FixtureGame {
    rules: Table,
    manifest: GameDataManifest,
    state: FixtureState,
    _lua: Lua,
}

impl FixtureGame {
    pub(crate) fn new(manifest: &str, modules: HashMap<String, String>) -> Result<Self, String> {
        let manifest: GameDataManifest =
            ron::from_str(manifest).map_err(|error| format!("{FIXTURE_MANIFEST_PATH}: {error}"))?;
        let lua = Lua::new();
        engine_core::scripting::install_prefetched(&lua, modules)
            .map_err(|error| format!("fixture modules: {error}"))?;
        let rules: Table = lua
            .load(format!("return require('{FIXTURE_ENTRY_MODULE}')"))
            .eval()
            .map_err(|error| format!("{FIXTURE_ENTRY_MODULE}: {error}"))?;
        let state = initial(&rules)?;
        Ok(Self {
            rules,
            manifest,
            state,
            _lua: lua,
        })
    }

    pub(crate) fn name(&self) -> &str {
        &self.manifest.name
    }

    pub(crate) fn state(&self) -> FixtureState {
        self.state
    }

    pub(crate) fn step(&mut self, dx: f32, dy: f32, dt: f32) -> Result<bool, String> {
        if self.state.won {
            return Ok(false);
        }
        let advance: Function = self
            .rules
            .get("advance")
            .map_err(|error| format!("{FIXTURE_ENTRY_MODULE}: {error}"))?;
        let (x, y, reached): (f32, f32, bool) = advance
            .call((self.state.x, self.state.y, dx, dy, dt))
            .map_err(|error| format!("{FIXTURE_ENTRY_MODULE}: {error}"))?;
        self.state.x = x;
        self.state.y = y;
        self.state.won = reached;
        Ok(reached)
    }

    pub(crate) fn reset(&mut self) -> Result<(), String> {
        self.state = initial(&self.rules)?;
        Ok(())
    }
}

fn initial(rules: &Table) -> Result<FixtureState, String> {
    let function: Function = rules
        .get("initial")
        .map_err(|error| format!("{FIXTURE_ENTRY_MODULE}: {error}"))?;
    let (x, y, goal_x, goal_y) = function
        .call(())
        .map_err(|error| format!("{FIXTURE_ENTRY_MODULE}: {error}"))?;
    Ok(FixtureState {
        x,
        y,
        goal_x,
        goal_y,
        won: false,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use engine_core::constants::paths;
    use web_shell::{
        FIXTURE_ASSET_ROOT, FIXTURE_MANIFEST_PATH, FIXTURE_MODULES, FIXTURE_NAME,
        fixture_module_path,
    };

    use super::FixtureGame;

    fn source_path(url: &str) -> PathBuf {
        let relative = url.strip_prefix(FIXTURE_ASSET_ROOT).unwrap();
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(paths::GAME_SAVE_ROOT)
            .join(FIXTURE_NAME)
            .join(paths::RESOURCES_FOLDER)
            .join(relative)
    }

    fn source(url: &str) -> String {
        fs::read_to_string(source_path(url)).unwrap()
    }

    fn sources() -> HashMap<String, String> {
        FIXTURE_MODULES
            .iter()
            .map(|name| {
                let url = fixture_module_path(name);
                (
                    url.strip_prefix(FIXTURE_ASSET_ROOT).unwrap().to_owned(),
                    source(&url),
                )
            })
            .collect()
    }

    #[test]
    fn fixture_axis_when_right_key_is_released_returns_to_neutral() {
        let mut input = bishop::input::KeyboardMouseState::new();
        input.on_key_down(bishop::input::KeyCode::D);
        assert_eq!(super::fixture_axis(&input), (1.0, 0.0));
        input.on_key_up(bishop::input::KeyCode::D);
        assert_eq!(super::fixture_axis(&input), (0.0, 0.0));
        input.on_key_down(bishop::input::KeyCode::Right);
        assert_eq!(super::fixture_axis(&input), (1.0, 0.0));
        input.on_key_up(bishop::input::KeyCode::Right);
        assert_eq!(super::fixture_axis(&input), (0.0, 0.0));
    }

    #[test]
    fn fixture_axis_when_opposite_keys_are_held_cancels_motion() {
        use bishop::input::{KeyCode, KeyboardMouseState};

        let mut input = KeyboardMouseState::new();
        input.on_key_down(KeyCode::A);
        input.on_key_down(KeyCode::D);
        input.on_key_down(KeyCode::Up);
        input.on_key_down(KeyCode::Down);
        assert_eq!(super::fixture_axis(&input), (0.0, 0.0));
        input.on_key_up(KeyCode::A);
        input.on_key_up(KeyCode::Down);
        assert_eq!(super::fixture_axis(&input), (1.0, -1.0));
    }

    #[test]
    fn fixture_when_created_reads_real_manifest_and_scripts() {
        let game = FixtureGame::new(&source(FIXTURE_MANIFEST_PATH), sources()).unwrap();
        assert_eq!(game.name(), FIXTURE_NAME);
        assert_eq!(game.state().x, 300.0);
        assert!(!game.state().won);
    }

    #[test]
    fn fixture_when_moving_right_advances_player() {
        let mut game = FixtureGame::new(&source(FIXTURE_MANIFEST_PATH), sources()).unwrap();
        let x = game.state().x;
        game.step(1.0, 0.0, 0.05).unwrap();
        assert!(game.state().x > x);
    }

    #[test]
    fn fixture_when_moving_far_stays_inside_the_canvas_bounds() {
        let mut game = FixtureGame::new(&source(FIXTURE_MANIFEST_PATH), sources()).unwrap();
        game.step(1.0, 1.0, 10.0).unwrap();
        let far_edge = game.state();
        assert_eq!((far_edge.x, far_edge.y), (620.0, 275.0));
        game.step(-1.0, -1.0, 10.0).unwrap();
        let near_edge = game.state();
        assert_eq!((near_edge.x, near_edge.y), (0.0, 65.0));
    }

    #[test]
    fn fixture_when_goal_reached_reports_one_win_transition() {
        let mut game = FixtureGame::new(&source(FIXTURE_MANIFEST_PATH), sources()).unwrap();
        let start = game.state();
        assert!(start.goal_x > start.x);
        assert_eq!(start.goal_y, start.y);

        let mut wins = 0;
        for _ in 0..40 {
            wins += u32::from(game.step(1.0, 0.0, 0.05).unwrap());
        }
        assert_eq!(wins, 1);
        assert!(game.state().won);
        assert!(!game.step(1.0, 0.0, 0.05).unwrap());
    }

    #[test]
    fn fixture_when_reset_restores_the_authored_initial_state() {
        let mut game = FixtureGame::new(&source(FIXTURE_MANIFEST_PATH), sources()).unwrap();
        let initial = game.state();
        for _ in 0..40 {
            game.step(1.0, 0.0, 0.05).unwrap();
        }
        assert!(game.state().won);

        game.reset().unwrap();
        assert_eq!(game.state(), initial);
    }

    #[test]
    fn fixture_when_rule_module_is_missing_reports_require_error() {
        let mut files = sources();
        let url = fixture_module_path(FIXTURE_MODULES[1]);
        files.remove(url.strip_prefix(FIXTURE_ASSET_ROOT).unwrap());
        let error = FixtureGame::new(&source(FIXTURE_MANIFEST_PATH), files)
            .err()
            .unwrap();
        assert!(error.contains(FIXTURE_MODULES[1]));
    }

    #[test]
    fn fixture_when_manifest_is_invalid_reports_file_name() {
        let error = FixtureGame::new("not RON", sources()).err().unwrap();
        let manifest_name = Path::new(FIXTURE_MANIFEST_PATH)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        assert!(error.contains(manifest_name));
    }
}
