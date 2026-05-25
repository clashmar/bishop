// game/src/playtest_main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::prelude::*;
use game_lib::engine::Engine;
use game_lib::startup::{
    runtime_icon_for_playtest_payload, PlaytestLaunchArgs, StartupController, StartupRequest,
};
use std::env;

/// Wrapper struct for running playtest via BishopApp.
struct PlaytestApp {
    payload_path: String,
    engine: Option<Engine>,
    startup: Option<StartupController>,
    current_startup_request: Option<StartupRequest>,
}

impl PlaytestApp {
    fn new(payload_path: String) -> Self {
        Self {
            payload_path,
            engine: None,
            startup: None,
            current_startup_request: None,
        }
    }
}

impl BishopApp for PlaytestApp {
    async fn init(&mut self, ctx: PlatformContext) {
        set_engine_mode(EngineMode::Playtest);
        let _ = ctx;
        let request = StartupRequest::playtest(self.payload_path.clone());
        self.current_startup_request = Some(request.clone());
        self.startup = Some(StartupController::from_request(request));
    }

    async fn frame(&mut self, ctx: PlatformContext) {
        if let Some(engine) = &mut self.engine {
            engine.frame(ctx.clone()).await;

            if let Some(load_request) = engine.save_runtime.take_pending_runtime_load_request() {
                let playtest_for_fallback = || {
                    StartupRequest::playtest(self.payload_path.clone())
                };
                let current = self
                    .current_startup_request
                    .clone()
                    .unwrap_or_else(playtest_for_fallback);
                let next_request = current.for_runtime_load(load_request);
                self.current_startup_request = Some(next_request.clone());
                self.engine = None;
                self.startup = Some(StartupController::from_request(next_request));
            }
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
    set_engine_mode(EngineMode::Playtest);

    let args: Vec<String> = env::args().collect();
    let launch_args = match PlaytestLaunchArgs::parse(&args) {
        Ok(args) => args,
        Err(usage) => {
            eprintln!("{usage}");
            std::process::exit(1);
        }
    };

    let mut config = WindowConfig::new("Playtest").with_fullscreen(true);
    if let Some(icon) = runtime_icon_for_playtest_payload(&launch_args.payload_path) {
        config = config.with_icon(icon);
    }
    // .with_size(width as u32, height as u32)
    // .with_resizable(true);

    let app = PlaytestApp::new(launch_args.payload_path);
    run_backend(config, app)
}
