// game/src/main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::prelude::*;
use game_lib::engine::Engine;
use game_lib::startup::{StartupController, StartupSource};
use std::env;
use std::fs;

/// Wrapper struct for running the game via BishopApp.
struct GameApp {
    engine: Option<Engine>,
    startup: Option<StartupController>,
}

impl GameApp {
    fn new() -> Self {
        Self {
            engine: None,
            startup: None,
        }
    }
}

impl BishopApp for GameApp {
    async fn init(&mut self, ctx: PlatformContext) {
        onscreen_info!("Initializing game.");
        let _ = ctx;
        self.startup = Some(StartupController::new(StartupSource::Game));
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

/// Helper that returns the icon from PNG bytes.
fn load_icon_from_png(png_bytes: &[u8]) -> WindowIcon {
    WindowIcon::Rgba {
        small: Some(IconData::new(
            load_rgba_resized::<{ 16 * 16 * 4 }>(png_bytes, 16).to_vec(),
            16,
            16,
        )),
        medium: Some(IconData::new(
            load_rgba_resized::<{ 32 * 32 * 4 }>(png_bytes, 32).to_vec(),
            32,
            32,
        )),
        large: Some(IconData::new(
            load_rgba_resized::<{ 64 * 64 * 4 }>(png_bytes, 64).to_vec(),
            64,
            64,
        )),
    }
}

fn main() -> Result<(), RunError> {
    // Load icon from resources directory if available
    let icon = resources_dir_from_exe()
        .and_then(|resources_dir| {
            let icon_path = resources_dir.join("Icon.png");
            fs::read(&icon_path).ok()
        })
        .map(|png_bytes| load_icon_from_png(&png_bytes));

    // Use the exe as the window title
    let window_title = env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Game".to_string());

    let mut config = WindowConfig::new(window_title)
        .with_fullscreen(true)
        .with_resizable(true);

    if let Some(icon) = icon {
        config = config.with_icon(icon);
    }

    let app = GameApp::new();
    run_backend(config, app)
}
