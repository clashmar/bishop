// game/src/playtest_main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::constants::agents;
use game_lib::agents::FileAgentSessionTransport;
use engine_core::agents::{
    AgentSessionManifest, AgentSessionRole, AgentSessionState, AgentSessionTransport,
    AgentVisibilitySnapshot,
};
use engine_core::prelude::*;
use engine_core::logging::{
    clear_agent_visibility_sink, publish_agent_visibility_snapshot, set_agent_visibility_sink,
    LOG_HISTORY,
};
use game_lib::engine::Engine;
use game_lib::startup::{
    runtime_icon_for_playtest_payload, PlaytestLaunchArgs, StartupController, StartupSource,
};
use std::env;
use std::path::PathBuf;
use uuid::Uuid;

mod playtest;

use playtest::headless::HeadlessPlaytestSession;

/// Wrapper struct for running playtest via BishopApp.
struct PlaytestApp {
    launch_args: PlaytestLaunchArgs,
    session_id: String,
    agent_transport: Option<FileAgentSessionTransport>,
    agent_manifest: Option<AgentSessionManifest>,
    frame_index: u64,
    engine: Option<Engine>,
    startup: Option<StartupController>,
    headless_session: Option<HeadlessPlaytestSession>,
}

struct PlaytestRuntimePayload {
    accumulator_ms: f32,
    player_velocity_x: f32,
}

impl PlaytestRuntimePayload {
    fn into_ron_value(self) -> ron::Value {
        [
            ("accumulator_ms", ron::Value::from(self.accumulator_ms)),
            ("player_velocity_x", ron::Value::from(self.player_velocity_x)),
        ]
        .into_iter()
        .collect()
    }
}

impl PlaytestApp {
    fn new(launch_args: PlaytestLaunchArgs) -> Self {
        Self {
            launch_args,
            session_id: Uuid::new_v4().to_string(),
            agent_transport: None,
            agent_manifest: None,
            frame_index: 0,
            engine: None,
            startup: None,
            headless_session: None,
        }
    }
}

impl BishopApp for PlaytestApp {
    async fn init(&mut self, ctx: PlatformContext) {
        set_engine_mode(EngineMode::Playtest);
        let _ = ctx;
        let transport = FileAgentSessionTransport::new(session_dir_for_launch(
            &self.launch_args,
            &self.session_id,
        ));
        set_agent_visibility_sink(Box::new(transport.clone()));
        let manifest = AgentSessionManifest {
            session_id: self.session_id.clone(),
            role: AgentSessionRole::Playtest,
            state: AgentSessionState::Starting,
            payload_path: self
                .launch_args
                .agent_payload_path
                .clone()
                .or(self.launch_args.payload_path.clone()),
            log_path: Some(agents::PLAYTEST_LOG_PATH.to_string()),
        };
        if let Err(e) = transport.write_manifest(&manifest) {
            onscreen_error!("Failed to write agent session manifest: {e}");
        }
        self.agent_manifest = Some(manifest);
        self.agent_transport = Some(transport);
        if let Some(payload_path) = self.launch_args.agent_payload_path.clone() {
            self.startup = Some(StartupController::new(StartupSource::AgentPayload {
                payload_path,
            }));
        } else if self.launch_args.headless {
            self.headless_session = Some(HeadlessPlaytestSession::new(self.session_id.clone()));
        } else {
            let payload_path = self
                .launch_args
                .payload_path
                .clone()
                .expect("payload path required for editor-attached playtest");
            self.startup = Some(StartupController::new(StartupSource::Playtest { payload_path }));
        }
    }

    async fn frame(&mut self, ctx: PlatformContext) {
        if let Some(engine) = &mut self.engine {
            engine.frame(ctx.clone()).await;
            let frame_index = self.frame_index;
            let snapshot = Self::make_snapshot(&ctx, engine, frame_index);
            publish_agent_visibility_snapshot(snapshot);
            self.frame_index += 1;
            return;
        }

        if let Some(headless_session) = &self.headless_session {
            let engine = headless_session.build_engine(ctx.clone());
            self.update_manifest_state(AgentSessionState::Running);
            self.engine = Some(engine);
            self.headless_session = None;
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

impl Drop for PlaytestApp {
    fn drop(&mut self) {
        clear_agent_visibility_sink();
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
    if let Some(payload_path) = launch_args
        .agent_payload_path
        .as_ref()
        .or(launch_args.payload_path.as_ref())
    {
        if let Some(icon) = runtime_icon_for_playtest_payload(payload_path) {
            config = config.with_icon(icon);
        }
    }
    // .with_size(width as u32, height as u32)
    // .with_resizable(true);

    let app = PlaytestApp::new(launch_args);
    run_backend(config, app)
}

fn session_dir_for_launch(launch_args: &PlaytestLaunchArgs, session_id: &str) -> PathBuf {
    let payload_path = launch_args
        .agent_payload_path
        .as_deref()
        .or(launch_args.payload_path.as_deref())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(session_id));
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

    fn make_snapshot(
        ctx: &PlatformContext,
        engine: &Engine,
        frame_index: u64,
    ) -> AgentVisibilitySnapshot {
        let frame_time_ms = ctx.borrow().get_frame_time() * 1000.0;
        let smoothed_frame_time_ms = engine.smoothed_dt.map(|dt| dt * 1000.0);
        let game_instance = engine.game_instance.borrow();
        let ecs = &game_instance.game.ecs;
        let player_entity = ecs.get_player_entity();
        let player_velocity_x = player_entity
            .and_then(|entity| ecs.get::<Velocity>(entity).copied())
            .map(|velocity| velocity.x)
            .unwrap_or_default();
        let recent_log_count = match LOG_HISTORY.lock() {
            Ok(history) => history.total_pushed(),
            Err(_) => 0,
        };

        AgentVisibilitySnapshot {
            session_state: AgentSessionState::Running,
            frame_time_ms: Some(frame_time_ms),
            smoothed_frame_time_ms,
            mode: Some(agents::PLAYTEST_MODE.to_string()),
            recent_log_count,
            frame_index: Some(frame_index),
            topic: Some(agents::PLAYTEST_RUNTIME_TOPIC.to_string()),
            label: Some(agents::PLAYTEST_FRAME_LABEL.to_string()),
            payload: Some(
                PlaytestRuntimePayload {
                    accumulator_ms: engine.accumulator * 1000.0,
                    player_velocity_x,
                }
                .into_ron_value(),
            ),
        }
    }

}
