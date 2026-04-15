// game/src/playtest_main.rs
use bishop::prelude::*;
use bishop::BishopApp;
use crate::playtest::FilePlaytestSessionTransport;
use engine_core::playtest::{PlaytestSessionState, PlaytestSnapshotRequest};
use engine_core::prelude::*;
use game_lib::engine::Engine;
use game_lib::game_global::{
    finalize_completed_playtest_control_frame, has_active_playtest_control_timeline,
    has_pending_completed_playtest_control_frame, install_active_playtest_control_timeline,
};
use game_lib::playtest::control::{AcceptedPlaytestControlRequest, ActiveControlTimeline};
use game_lib::startup::{runtime_icon_for_playtest_payload, PlaytestLaunchArgs, StartupController, StartupSource};
use std::env;
use uuid::Uuid;

mod playtest;
#[path = "playtest_main/session.rs"]
mod session;
#[path = "playtest_main/snapshot.rs"]
mod snapshot;
#[cfg(test)]
#[path = "playtest_main/tests.rs"]
mod tests;

use self::session::{session_dir_for_launch, PlaytestRuntimeSession};
use self::snapshot::{advance_active_control_runtime_for_next_snapshot, make_snapshot, ActiveControlSnapshotState};

/// Wrapper struct for running playtest via BishopApp.
struct PlaytestApp {
    launch_args: PlaytestLaunchArgs,
    session: PlaytestRuntimeSession,
    active_snapshot_request: PlaytestSnapshotRequest,
    active_control_request: Option<AcceptedPlaytestControlRequest>,
    active_control_runtime: Option<ActiveControlSnapshotState>,
    frame_index: u64,
    engine: Option<Engine>,
    startup: Option<StartupController>,
}

impl PlaytestApp {
    fn new(launch_args: PlaytestLaunchArgs) -> Self {
        let active_snapshot_request = PlaytestSnapshotRequest::default();
        let active_control_request: Option<AcceptedPlaytestControlRequest> = None;
        let active_control_runtime = active_control_request
            .as_ref()
            .and_then(|accepted| ActiveControlSnapshotState::from_accepted(accepted.clone()));
        let session_id = Uuid::new_v4().to_string();
        let session = PlaytestRuntimeSession::unattached(
            session_id.clone(),
            active_snapshot_request.clone(),
            active_control_request.clone(),
        );

        Self {
            session,
            active_snapshot_request,
            active_control_request,
            active_control_runtime,
            launch_args,
            frame_index: 0,
            engine: None,
            startup: None,
        }
    }
}

impl BishopApp for PlaytestApp {
    async fn init(&mut self, ctx: PlatformContext) {
        set_engine_mode(EngineMode::Playtest);
        let _ = ctx;
        self.session.attach_transport(FilePlaytestSessionTransport::new(
            session_dir_for_launch(&self.launch_args),
        ));
        self.session
            .initialize_manifest(self.launch_args.payload_path.clone());
        if let Some(accepted) = self.active_control_request.as_ref() {
            self.session.persist_expanded_control_profile(accepted);
        }
        self.startup = Some(StartupController::new(StartupSource::Playtest {
            payload_path: self.launch_args.payload_path.clone(),
        }));
    }

    async fn frame(&mut self, ctx: PlatformContext) {
        if self.engine.is_some() {
            self.poll_runtime_requests();
        }

        if let Some(engine) = &mut self.engine {
            engine.frame(ctx.clone()).await;
            let frame_index = self.frame_index;
            let game_state = engine.game_state.clone();
            let snapshot = make_snapshot(
                &ctx,
                engine,
                frame_index,
                &self.active_snapshot_request,
                self.active_control_runtime.as_ref(),
            );
            self.session.write_snapshot(&snapshot);
            advance_active_control_runtime_for_next_snapshot(
                &mut self.active_control_runtime,
                &game_state,
            );
            finalize_completed_playtest_control_frame();
            self.frame_index += 1;
        } else {
            if let Some(startup) = &mut self.startup {
                if let Some(engine) = startup.frame(ctx).await {
                    self.engine = Some(engine);
                    self.update_manifest_state(PlaytestSessionState::Running);
                    self.startup = None;
                }
            }
            return;
        }

        if self.active_control_request.is_some()
            && !has_active_playtest_control_timeline()
            && !has_pending_completed_playtest_control_frame()
        {
            self.active_control_runtime = None;
            self.active_control_request = None;
            self.session.clear_active_control();
            if let Some(engine) = &mut self.engine {
                engine.clear_playtest_camera_overrides();
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

    let app = PlaytestApp::new(launch_args);
    run_backend(config, app)
}

impl PlaytestApp {
    fn update_manifest_state(&mut self, state: PlaytestSessionState) {
        self.session.update_manifest_state(state);
    }

    fn poll_runtime_requests(&mut self) {
        if let Some(accepted) = self.session.poll_runtime_requests() {
            self.active_snapshot_request = self.session.snapshot_request().clone();
            install_active_playtest_control_timeline(ActiveControlTimeline::new(accepted.profile.clone()));
            self.active_control_runtime = ActiveControlSnapshotState::from_accepted(accepted.clone());
            self.active_control_request = Some(accepted);
        } else {
            self.active_snapshot_request = self.session.snapshot_request().clone();
        }
    }
}
