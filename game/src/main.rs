// game/src/main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::prelude::*;
use game_lib::engine::{Engine, EngineBuilder, GameInstance};
use std::env;
use std::fs;

/// Wrapper struct for running the game via BishopApp.
struct GameApp {
    engine: Option<Engine>,
    ctx: Option<PlatformContext>,
}

impl GameApp {
    fn new() -> Self {
        Self {
            engine: None,
            ctx: None,
        }
    }
}

impl BishopApp for GameApp {
    async fn init(&mut self, ctx: PlatformContext) {
        onscreen_info!("Initializing game.");
        self.ctx = Some(ctx.clone());

        let mut builder = EngineBuilder::new();

        let game_instance = {
            let mut ctx_ref = ctx.borrow_mut();
            GameInstance::new(&mut *ctx_ref, &builder.lua, &mut builder.camera_manager)
        };

        self.engine = Some(builder.assemble(game_instance, ctx, false));
    }

    async fn frame(&mut self, ctx: PlatformContext) {
        if let Some(engine) = &mut self.engine {
            engine.frame(ctx).await;
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
