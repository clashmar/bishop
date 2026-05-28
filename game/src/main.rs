// game/src/main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::prelude::*;
use game_lib::engine::Engine;
use game_lib::startup::{runtime_icon_for_current_exe, StartupController, StartupIntent, StartupRequest};
use std::any::Any;
use std::env;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

/// Wrapper struct for running the game via BishopApp.
struct GameApp {
    engine: Option<Engine>,
    startup: Option<StartupController>,
    current_startup_request: Option<StartupRequest>,
}

impl GameApp {
    fn new() -> Self {
        Self {
            engine: None,
            startup: None,
            current_startup_request: None,
        }
    }
}

impl BishopApp for GameApp {
    async fn init(&mut self, ctx: PlatformContext) {
        onscreen_info!("Initializing game.");
        let _ = ctx;
        let request = StartupRequest::game();
        self.current_startup_request = Some(request.clone());
        self.startup = Some(StartupController::from_request(request));
    }

    async fn frame(&mut self, ctx: PlatformContext) {
        if let Some(engine) = &mut self.engine {
            engine.frame(ctx.clone()).await;
            self.maybe_rebootstrap();
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

impl GameApp {
    fn maybe_rebootstrap(&mut self) {
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };

        let next_request = if engine.save_runtime.pending_quit_to_title.get() {
            engine.save_runtime.pending_quit_to_title.set(false);
            StartupRequest {
                intent: StartupIntent::QuitToTitle,
                ..StartupRequest::game()
            }
        } else if let Some(load_request) = engine.save_runtime.take_pending_runtime_load_request()
        {
            self.current_startup_request
                .clone()
                .unwrap_or_else(StartupRequest::game)
                .for_runtime_load(load_request)
        } else {
            return;
        };

        self.current_startup_request = Some(next_request.clone());
        self.engine = None;
        self.startup = Some(StartupController::from_request(next_request));
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

    let icon = runtime_icon_for_current_exe();

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
