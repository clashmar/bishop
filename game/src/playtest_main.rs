// game/src/playtest_main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::payload;
use engine_core::constants::agents;
use game_lib::agents::FileAgentSessionTransport;
use engine_core::agents::{
    build_snapshot_payload, payload_value, AgentSessionManifest, AgentSessionRole,
    AgentSessionState, AgentSnapshotRequest, SnapshotProfile, AgentSessionTransport,
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
use serde::Serialize;
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
    active_snapshot_request: AgentSnapshotRequest,
    frame_index: u64,
    engine: Option<Engine>,
    startup: Option<StartupController>,
    headless_session: Option<HeadlessPlaytestSession>,
}

#[derive(Serialize)]
struct PlaytestRuntimePayload {
    accumulator_ms: f32,
    player_velocity_x: f32,
}

impl PlaytestApp {
    fn new(launch_args: PlaytestLaunchArgs) -> Self {
        Self {
            active_snapshot_request: Self::launch_snapshot_request(&launch_args),
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
            snapshot_request: Some(self.active_snapshot_request.clone()),
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
        if self.engine.is_some() {
            self.poll_runtime_request();
        }

        if let Some(engine) = &mut self.engine {
            engine.frame(ctx.clone()).await;
            let frame_index = self.frame_index;
            let snapshot = Self::make_snapshot(
                &ctx,
                engine,
                frame_index,
                &self.active_snapshot_request,
            );
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
        request: &AgentSnapshotRequest,
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
        let runtime_payload = match payload_value(PlaytestRuntimePayload {
            accumulator_ms: engine.accumulator * 1000.0,
            player_velocity_x,
        }) {
            Ok(payload) => Some(payload),
            Err(error) => {
                onscreen_error!("Failed to serialize playtest runtime payload: {error:?}");
                None
            }
        };
        let payload = build_snapshot_payload(request, runtime_payload);

        AgentVisibilitySnapshot {
            session_state: AgentSessionState::Running,
            frame_time_ms: Some(frame_time_ms),
            smoothed_frame_time_ms,
            mode: Some(agents::PLAYTEST_MODE.to_string()),
            recent_log_count,
            frame_index: Some(frame_index),
            topic: Some(agents::PLAYTEST_RUNTIME_TOPIC.to_string()),
            label: Some(agents::PLAYTEST_FRAME_LABEL.to_string()),
            payload,
        }
    }

    fn launch_snapshot_request(launch_args: &PlaytestLaunchArgs) -> AgentSnapshotRequest {
        launch_args
            .agent_payload_path
            .as_deref()
            .and_then(Self::load_snapshot_request_from_payload)
            .unwrap_or_else(Self::default_snapshot_request)
    }

    fn default_snapshot_request() -> AgentSnapshotRequest {
        AgentSnapshotRequest {
            profile: SnapshotProfile::Minimal,
            extras: payload!(),
        }
    }

    fn load_snapshot_request_from_payload(payload_path: &str) -> Option<AgentSnapshotRequest> {
        game_lib::agents::load_agent_payload(payload_path)
            .ok()
            .and_then(|payload| payload.snapshot_request)
    }

    fn poll_runtime_request(&mut self) {
        let Some(transport) = self.agent_transport.as_ref() else {
            return;
        };

        if let Some(request) = Self::consume_runtime_request(transport) {
            self.active_snapshot_request = request.clone();
            self.update_manifest_snapshot_request(request);
        }
    }

    fn update_manifest_snapshot_request(&mut self, request: AgentSnapshotRequest) {
        let (Some(transport), Some(mut manifest)) = (
            self.agent_transport.as_ref(),
            self.agent_manifest.clone(),
        ) else {
            return;
        };

        manifest.snapshot_request = Some(request);
        if let Err(e) = transport.write_manifest(&manifest) {
            onscreen_error!("Failed to update agent session manifest: {e}");
            return;
        }

        self.agent_manifest = Some(manifest);
    }

    fn consume_runtime_request(
        transport: &FileAgentSessionTransport,
    ) -> Option<AgentSnapshotRequest> {
        let path = transport.request_path();
        let ron = std::fs::read_to_string(&path).ok()?;
        let request = ron::from_str::<AgentSnapshotRequest>(&ron).ok()?;
        let _ = std::fs::remove_file(&path);
        Some(request)
    }
}

#[cfg(test)]
mod tests {
    use super::PlaytestApp;
    use engine_core::agents::visibility::{AgentSnapshotRequest, SnapshotProfile};
    use engine_core::payload;
    use game_lib::agents::FileAgentSessionTransport;
    use game_lib::startup::PlaytestLaunchArgs;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn valid_runtime_request_is_consumed_and_applied() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let session_dir = std::env::temp_dir().join(format!("playtest_runtime_request_{unique}"));
        let transport = FileAgentSessionTransport::new(session_dir.clone());
        let request = AgentSnapshotRequest {
            profile: SnapshotProfile::RuntimeDebug,
            extras: payload!(player_velocity_x: 2.5),
        };
        let mut app = PlaytestApp::new(PlaytestLaunchArgs {
            payload_path: None,
            agent_payload_path: None,
            headless: true,
        });
        app.agent_transport = Some(transport.clone());

        assert_eq!(app.active_snapshot_request.profile, SnapshotProfile::Minimal);
        fs::create_dir_all(&session_dir).unwrap();
        fs::write(
            transport.request_path(),
            ron::ser::to_string_pretty(&request, ron::ser::PrettyConfig::default()).unwrap(),
        )
        .unwrap();

        app.poll_runtime_request();

        assert_eq!(app.active_snapshot_request, request);
        assert!(!transport.request_path().exists());

        let _ = fs::remove_dir_all(session_dir);
    }

    #[test]
    fn manifest_mirrors_latest_accepted_snapshot_request() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let session_dir = std::env::temp_dir().join(format!("playtest_manifest_request_{unique}"));
        let transport = FileAgentSessionTransport::new(session_dir.clone());
        let initial_request = AgentSnapshotRequest {
            profile: SnapshotProfile::Minimal,
            extras: payload!(),
        };
        let mut app = PlaytestApp::new(PlaytestLaunchArgs {
            payload_path: None,
            agent_payload_path: None,
            headless: true,
        });
        app.agent_transport = Some(transport.clone());
        app.agent_manifest = Some(engine_core::agents::AgentSessionManifest {
            session_id: "session-1".to_string(),
            role: engine_core::agents::AgentSessionRole::Playtest,
            state: engine_core::agents::AgentSessionState::Starting,
            payload_path: None,
            log_path: None,
            snapshot_request: Some(initial_request),
        });

        let request = AgentSnapshotRequest {
            profile: SnapshotProfile::RuntimeDebug,
            extras: payload!(player_velocity_x: 2.5),
        };

        fs::create_dir_all(&session_dir).unwrap();
        fs::write(
            transport.request_path(),
            ron::ser::to_string_pretty(&request, ron::ser::PrettyConfig::default()).unwrap(),
        )
        .unwrap();

        app.poll_runtime_request();

        let manifest_ron = fs::read_to_string(transport.manifest_path()).unwrap();
        let manifest: engine_core::agents::AgentSessionManifest =
            ron::from_str(&manifest_ron).unwrap();
        assert_eq!(manifest.snapshot_request, Some(request));

        let _ = fs::remove_dir_all(session_dir);
    }
}
