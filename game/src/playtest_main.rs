// game/src/playtest_main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::prelude::*;
use game_lib::engine::{Engine, EngineBuilder, GameInstance};
use ron::de::from_str;
use std::{env, fs};

/// The complete payload the editor writes for the play-test binary.
#[derive(serde::Deserialize)]
struct PlaytestPayload {
    room: Room,
    game: Game,
}

/// Wrapper struct for running playtest via BishopApp.
struct PlaytestApp {
    payload_path: String,
    engine: Option<Engine>,
    ctx: Option<PlatformContext>,
}

impl PlaytestApp {
    fn new(payload_path: String) -> Self {
        Self {
            payload_path,
            engine: None,
            ctx: None,
        }
    }
}

impl BishopApp for PlaytestApp {
    async fn init(&mut self, ctx: PlatformContext) {
        let payload_str = match fs::read_to_string(&self.payload_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Could not read the temporary playtest file: {e}");
                std::process::exit(1);
            }
        };

        let PlaytestPayload { room, game } = match from_str(&payload_str) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to deserialize playtest payload: {e}");
                std::process::exit(1);
            }
        };

        self.ctx = Some(ctx.clone());
        set_engine_mode(EngineMode::Playtest);

        let mut builder = EngineBuilder::new();

        let game_instance = {
            let mut ctx_ref = ctx.borrow_mut();
            GameInstance::for_room(
                &mut *ctx_ref,
                room,
                game,
                &builder.lua,
                &mut builder.camera_manager,
            )
        };

        self.engine = Some(builder.assemble(game_instance, ctx, true));
    }

    async fn frame(&mut self, ctx: PlatformContext) {
        if let Some(engine) = &mut self.engine {
            engine.frame(ctx).await;
        }
    }
}

fn main() -> Result<(), RunError> {
    // Load the temporary file written by the editor
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <playtest_payload.ron>", args[0]);
        std::process::exit(1);
    }

    let payload_path = args[1].clone();

    let config = WindowConfig::new("Playtest").with_fullscreen(true);
    // .with_size(width as u32, height as u32)
    // .with_resizable(true);

    let app = PlaytestApp::new(payload_path);
    run_backend(config, app)
}
