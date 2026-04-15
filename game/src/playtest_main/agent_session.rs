use crate::playtest::FilePlaytestSessionTransport;
use engine_core::agents::AgentPlaytestControlRequest;
use engine_core::onscreen_error;
use engine_core::playtest::{
    PlaytestActiveControl, PlaytestSessionManifest, PlaytestSessionRole, PlaytestSessionState,
    PlaytestSessionTransport, PlaytestSnapshot, PlaytestSnapshotRequest,
};
use game_lib::playtest::control::{
    accept_playtest_control_request, AcceptedPlaytestControlRequest,
};
use game_lib::startup::PlaytestLaunchArgs;
use std::path::PathBuf;

pub(crate) struct AgentPlaytestSession {
    session_id: String,
    transport: Option<FilePlaytestSessionTransport>,
    manifest: Option<PlaytestSessionManifest>,
    active_snapshot_request: PlaytestSnapshotRequest,
    active_control_request: Option<AcceptedPlaytestControlRequest>,
}

impl AgentPlaytestSession {
    pub(crate) fn unattached(
        session_id: String,
        active_snapshot_request: PlaytestSnapshotRequest,
        active_control_request: Option<AcceptedPlaytestControlRequest>,
    ) -> Self {
        Self {
            session_id,
            transport: None,
            manifest: None,
            active_snapshot_request,
            active_control_request,
        }
    }

    pub(crate) fn attach_transport(&mut self, transport: FilePlaytestSessionTransport) {
        self.transport = Some(transport);
    }

    pub(crate) fn snapshot_request(&self) -> &PlaytestSnapshotRequest {
        &self.active_snapshot_request
    }

    pub(crate) fn write_snapshot(&self, snapshot: &PlaytestSnapshot) {
        let Some(transport) = self.transport.as_ref() else {
            return;
        };

        if let Err(error) = transport.write_snapshot(snapshot) {
            onscreen_error!("Failed to write agent snapshot: {error}");
        }
    }

    pub(crate) fn initialize_manifest(&mut self, payload_path: String) {
        let Some(transport) = self.transport.as_ref() else {
            return;
        };

        let manifest = PlaytestSessionManifest {
            session_id: self.session_id.clone(),
            role: PlaytestSessionRole::Playtest,
            state: PlaytestSessionState::Starting,
            payload_path: Some(payload_path),
            snapshot_request: Some(self.active_snapshot_request.clone()),
            active_control: self.active_control_request.clone().map(|accepted| {
                PlaytestActiveControl {
                    request: accepted.request,
                }
            }),
        };
        if let Err(error) = transport.write_manifest(&manifest) {
            onscreen_error!("Failed to write agent session manifest: {error}");
            return;
        }

        self.manifest = Some(manifest);
    }

    pub(crate) fn update_manifest_state(&mut self, state: PlaytestSessionState) {
        let (Some(transport), Some(mut manifest)) =
            (self.transport.as_ref(), self.manifest.clone())
        else {
            return;
        };

        manifest.state = state;
        if let Err(error) = transport.write_manifest(&manifest) {
            onscreen_error!("Failed to update agent session manifest: {error}");
            return;
        }

        self.manifest = Some(manifest);
    }

    pub(crate) fn poll_runtime_requests(&mut self) -> Option<AcceptedPlaytestControlRequest> {
        let transport = self.transport.clone()?;

        if let Some(request) = Self::consume_snapshot_runtime_request(&transport) {
            self.active_snapshot_request = request.clone();
            self.update_manifest_snapshot_request(request);
        }

        let request = Self::consume_control_runtime_request(&transport)?;
        self.apply_runtime_control_request(request)
    }

    pub(crate) fn apply_runtime_control_request(
        &mut self,
        request: AgentPlaytestControlRequest,
    ) -> Option<AcceptedPlaytestControlRequest> {
        let accepted = accept_playtest_control_request(request)?;
        self.apply_runtime_control(accepted.clone());
        Some(accepted)
    }

    pub(crate) fn apply_runtime_control(&mut self, accepted: AcceptedPlaytestControlRequest) {
        self.persist_expanded_control_profile(&accepted);
        self.active_control_request = Some(accepted.clone());
        self.update_manifest_active_control(Some(accepted.request));
    }

    pub(crate) fn clear_active_control(&mut self) {
        self.active_control_request = None;
        self.update_manifest_active_control(None);
    }

    pub(crate) fn persist_expanded_control_profile(
        &self,
        accepted: &AcceptedPlaytestControlRequest,
    ) {
        if accepted.request.chaos.is_none() {
            return;
        }

        if let Some(transport) = self.transport.as_ref() {
            if let Err(error) = transport.ensure_ready() {
                onscreen_error!("Failed to prepare expanded control artifact: {error}");
                return;
            }

            match ron::ser::to_string_pretty(&accepted.profile, ron::ser::PrettyConfig::default()) {
                Ok(ron) => {
                    if let Err(error) = std::fs::write(transport.expanded_control_path(), ron) {
                        onscreen_error!("Failed to persist expanded control artifact: {error}");
                    }
                }
                Err(error) => {
                    onscreen_error!("Failed to serialize expanded control artifact: {error}");
                }
            }
        }
    }

    pub(crate) fn consume_control_runtime_request(
        transport: &FilePlaytestSessionTransport,
    ) -> Option<AgentPlaytestControlRequest> {
        let path = transport.control_request_path();
        let ron = std::fs::read_to_string(&path).ok()?;
        match ron::from_str::<AgentPlaytestControlRequest>(&ron) {
            Ok(request) => {
                let _ = std::fs::remove_file(&path);
                Some(request)
            }
            Err(error) => {
                onscreen_error!("Failed to parse runtime control request: {error}");
                None
            }
        }
    }

    fn update_manifest_snapshot_request(&mut self, request: PlaytestSnapshotRequest) {
        let (Some(transport), Some(mut manifest)) =
            (self.transport.as_ref(), self.manifest.clone())
        else {
            return;
        };

        manifest.snapshot_request = Some(request);
        if let Err(error) = transport.write_manifest(&manifest) {
            onscreen_error!("Failed to update agent session manifest: {error}");
            return;
        }

        self.manifest = Some(manifest);
    }

    fn update_manifest_active_control(&mut self, request: Option<AgentPlaytestControlRequest>) {
        let (Some(transport), Some(mut manifest)) =
            (self.transport.as_ref(), self.manifest.clone())
        else {
            return;
        };

        manifest.active_control = request.map(|request| PlaytestActiveControl { request });
        if let Err(error) = transport.write_manifest(&manifest) {
            onscreen_error!("Failed to update agent session manifest: {error}");
            return;
        }

        self.manifest = Some(manifest);
    }

    fn consume_snapshot_runtime_request(
        transport: &FilePlaytestSessionTransport,
    ) -> Option<PlaytestSnapshotRequest> {
        let path = transport.request_path();
        let ron = std::fs::read_to_string(&path).ok()?;
        let request = ron::from_str::<PlaytestSnapshotRequest>(&ron).ok()?;
        let _ = std::fs::remove_file(&path);
        Some(request)
    }
}

pub(crate) fn session_dir_for_launch(launch_args: &PlaytestLaunchArgs) -> PathBuf {
    let payload_path = PathBuf::from(&launch_args.payload_path);
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
