// game/src/main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::prelude::*;
use game_lib::engine::Engine;
use game_lib::startup::{StartupController, StartupSource};
use std::any::Any;
use std::env;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

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
    let exe_path = env::current_exe().ok();
    let window_title = exe_path
        .as_ref()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Game".to_string());
    let telemetry = init_runtime_telemetry(&window_title);

    onscreen_info!("Launching game '{}'.", window_title);
    onscreen_info!("Runtime logs: {}", telemetry.log_dir.display());

    if let Some(exe_path) = &exe_path {
        onscreen_info!("Executable path: {}", exe_path.display());
    }

    // Load icon from resources directory if available
    let resources_dir = resources_dir_from_exe();

    let icon = resources_dir
        .as_ref()
        .and_then(|resources_dir| {
            let icon_path = resources_dir.join("Icon.png");
            fs::read(&icon_path).ok()
        })
        .map(|png_bytes| load_icon_from_png(&png_bytes));

    let mut config = WindowConfig::new(window_title)
        .with_fullscreen(true)
        .with_resizable(true);

    if let Some(icon) = icon {
        config = config.with_icon(icon);
    }

    let app = GameApp::new();
    run_with_global_error_handler(config, app, &telemetry.log_dir)
}

fn run_with_global_error_handler(
    config: WindowConfig,
    app: GameApp,
    log_dir: &Path,
) -> Result<(), RunError> {
    match catch_unwind(AssertUnwindSafe(|| run_backend(config, app))) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            show_fatal_error_dialog(
                "Game Failed",
                &fatal_panic_message(&error.to_string(), log_dir),
            );
            Err(error)
        }
        Err(payload) => {
            let message = fatal_panic_message(&panic_payload_message(payload.as_ref()), log_dir);
            show_fatal_error_dialog("Game Crashed", &message);
            std::process::exit(1);
        }
    }
}

fn show_fatal_error_dialog(title: &str, message: &str) {
    eprintln!("{message}");
    let _ = rfd::MessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

fn fatal_panic_message(message: &str, log_dir: &Path) -> String {
    format!("The game crashed.\n\n{message}\n\nSee logs in {}", log_dir.display())
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return (*message).to_string();
    }
    "Unknown panic payload".to_string()
}

#[cfg(test)]
mod tests {
    use super::{fatal_panic_message, panic_payload_message};
    use std::any::Any;
    use std::path::Path;

    #[test]
    fn fatal_panic_message_mentions_log_dir() {
        let message = fatal_panic_message("boom", Path::new("/tmp/bishop-logs"));

        assert!(message.contains("boom"));
        assert!(message.contains("/tmp/bishop-logs"));
    }

    #[test]
    fn panic_payload_message_reads_string_payload() {
        let payload: Box<dyn Any + Send> = Box::new(String::from("boom"));

        assert_eq!(panic_payload_message(payload.as_ref()), "boom");
    }
}
