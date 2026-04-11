// game/src/playtest_main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::agent_visibility::{
    AgentSessionManifest, AgentSessionRole, AgentSessionState, AgentSessionTransport,
    AgentVisibilitySnapshot,
};
use engine_core::prelude::*;
use engine_core::logging::LOG_HISTORY;
use game_lib::engine::Engine;
use game_lib::startup::{
    runtime_icon_for_playtest_payload, PlaytestLaunchArgs, StartupController, StartupSource,
};
use std::env;
use std::path::PathBuf;
use uuid::Uuid;

mod playtest;

use playtest::agent_visibility::FileAgentSessionTransport;

/// Wrapper struct for running playtest via BishopApp.
struct PlaytestApp {
    payload_path: String,
    session_id: String,
    agent_transport: Option<FileAgentSessionTransport>,
    agent_manifest: Option<AgentSessionManifest>,
    engine: Option<Engine>,
    startup: Option<StartupController>,
}

impl PlaytestApp {
    fn new(payload_path: String) -> Self {
        Self {
            payload_path,
            session_id: Uuid::new_v4().to_string(),
            agent_transport: None,
            agent_manifest: None,
            engine: None,
            startup: None,
        }
    }
}

impl BishopApp for PlaytestApp {
    async fn init(&mut self, ctx: PlatformContext) {
        set_engine_mode(EngineMode::Playtest);
        let _ = ctx;
        let transport = FileAgentSessionTransport::new(session_dir_for_payload(&self.payload_path));
        let manifest = AgentSessionManifest {
            session_id: self.session_id.clone(),
            role: AgentSessionRole::Playtest,
            state: AgentSessionState::Starting,
            payload_path: Some(self.payload_path.clone()),
            log_path: None,
        };
        if let Err(e) = transport.write_manifest(&manifest) {
            onscreen_error!("Failed to write agent session manifest: {e}");
        }
        self.agent_manifest = Some(manifest);
        self.agent_transport = Some(transport);
        self.startup = Some(StartupController::new(StartupSource::Playtest {
            payload_path: self.payload_path.clone(),
        }));
    }

    async fn frame(&mut self, ctx: PlatformContext) {
        if let Some(engine) = &mut self.engine {
            engine.frame(ctx.clone()).await;
            let snapshot = Self::make_snapshot(&ctx, engine);
            self.publish_snapshot(snapshot);
            return;
        }

        if let Some(startup) = &mut self.startup {
            if let Some(engine) = startup.frame(ctx).await {
                self.update_manifest_state(AgentSessionState::Running);
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
            onscreen_error!("{usage}");
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

fn session_dir_for_payload(payload_path: &str) -> PathBuf {
    let payload_path = PathBuf::from(payload_path);
    let session_dir_name = payload_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| format!("{stem}_agent"))
        .unwrap_or_else(|| "agent_session".to_string());
    payload_path
        .parent()
        .map(|parent| parent.join(&session_dir_name))
        .unwrap_or_else(|| PathBuf::from(session_dir_name))
}

impl PlaytestApp {
    fn update_manifest_state(&mut self, state: AgentSessionState) {
        let (Some(transport), Some(mut manifest)) = (
            self.agent_transport.as_ref(),
            self.agent_manifest.clone(),
        ) else {
            return;
        };

        manifest.state = state;
        if let Err(e) = transport.write_manifest(&manifest) {
            onscreen_error!("Failed to update agent session manifest: {e}");
            return;
        }

        self.agent_manifest = Some(manifest);
    }

    fn make_snapshot(ctx: &PlatformContext, engine: &Engine) -> AgentVisibilitySnapshot {
        let frame_time_ms = ctx.borrow().get_frame_time() * 1000.0;
        let smoothed_frame_time_ms = engine.smoothed_dt.map(|dt| dt * 1000.0);
        let recent_log_count = match LOG_HISTORY.lock() {
            Ok(history) => history.total_pushed(),
            Err(_) => 0,
        };

        AgentVisibilitySnapshot {
            session_state: AgentSessionState::Running,
            frame_time_ms: Some(frame_time_ms),
            smoothed_frame_time_ms,
            mode: Some("playtest".to_string()),
            recent_log_count,
        }
    }

    fn publish_snapshot(&self, snapshot: AgentVisibilitySnapshot) {
        let Some(transport) = self.agent_transport.as_ref() else {
            return;
        };

        if let Err(e) = transport.write_snapshot(&snapshot) {
            onscreen_error!("Failed to write agent snapshot: {e}");
        }
    }
}
