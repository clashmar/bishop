// game/src/playtest_main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::prelude::*;
use game_lib::engine::Engine;
use game_lib::startup::{PlaytestLaunchArgs, StartupController, StartupSource};
use std::env;

/// Wrapper struct for running playtest via BishopApp.
struct PlaytestApp {
    payload_path: String,
    engine: Option<Engine>,
    startup: Option<StartupController>,
}

impl PlaytestApp {
    fn new(payload_path: String) -> Self {
        Self {
            payload_path,
            engine: None,
            startup: None,
        }
    }
}

impl BishopApp for PlaytestApp {
    async fn init(&mut self, ctx: PlatformContext) {
        set_engine_mode(EngineMode::Playtest);
        let _ = ctx;
        self.startup = Some(StartupController::new(StartupSource::Playtest {
            payload_path: self.payload_path.clone(),
        }));
    }

    async fn frame(&mut self, ctx: PlatformContext) {
        if let Some(engine) = &mut self.engine {
            engine.frame(ctx).await;
            return;
        }

        if let Some(startup) = &mut self.startup {
            if let Some(engine) = startup.frame(ctx).await {
                self.engine = Some(engine);
                self.startup = None;
            }
        }
    }
}

fn main() -> Result<(), RunError> {
    let args: Vec<String> = env::args().collect();
    let launch_args = match PlaytestLaunchArgs::parse(&args) {
        Ok(args) => args,
        Err(usage) => {
            eprintln!("{usage}");
            std::process::exit(1);
        }
    };

    let config = WindowConfig::new("Playtest").with_fullscreen(true);
    // .with_size(width as u32, height as u32)
    // .with_resizable(true);

    let app = PlaytestApp::new(launch_args.payload_path);
    run_backend(config, app)
}
