use super::game_instance::GameInstance;
use crate::save_system::{
    capture_document, RuntimeSaveDocument, RuntimeSaveMetadata, SaveCoordinatorError, SaveLane,
    SaveProviderRegistry, SaveSlotKey, SaveStoreHandle, SaveWrite, RUNTIME_SAVE_SCHEMA_VERSION,
};
use crate::scripting::commands::save_commands::{emit_save_failed, emit_save_succeeded};
use engine_core::scripting::EventBus;
use mlua::Lua;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// The type of runtime load request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeLoadRequest {
    /// Load the most recent save across any lane.
    Latest,
}

struct PendingSave {
    lane: SaveLane,
    trigger: String,
}

/// Orchestrates runtime save execution and pending load requests.
pub struct SaveRuntime {
    providers: Rc<RefCell<SaveProviderRegistry<'static>>>,
    save_store: SaveStoreHandle,
    pending_saves: HashMap<u64, PendingSave>,
    pending_runtime_load_request: Option<RuntimeLoadRequest>,
    pub pending_quit_to_title: Rc<Cell<bool>>,
}

impl SaveRuntime {
    pub fn new(
        providers: Rc<RefCell<SaveProviderRegistry<'static>>>,
        pending_quit_to_title: Rc<Cell<bool>>,
        save_store: SaveStoreHandle,
    ) -> Self {
        Self {
            providers,
            save_store,
            pending_saves: HashMap::new(),
            pending_runtime_load_request: None,
            pending_quit_to_title,
        }
    }

    /// Captures state from all providers and submits it to `lane`.
    pub fn save_to_lane(
        &mut self,
        game_instance: &Rc<RefCell<GameInstance>>,
        lane: SaveLane,
    ) -> io::Result<SaveWrite> {
        let game = &game_instance.borrow().game;
        let metadata = RuntimeSaveMetadata {
            schema_version: RUNTIME_SAVE_SCHEMA_VERSION,
            game_id: game.id,
            game_name: game.name.clone(),
            lane,
            slot: SaveSlotKey::Default,
            saved_at_unix_ms: current_unix_ms(),
        };

        let document = capture_document(&mut self.providers.borrow_mut(), &metadata)
            .map_err(coordinator_to_io_error)?;
        self.save_store.borrow_mut().submit(document)
    }

    /// Retains a submitted lane and trigger until its store request completes.
    pub fn track_pending(&mut self, request_id: u64, lane: SaveLane, trigger: String) {
        self.pending_saves.insert(request_id, PendingSave { lane, trigger });
    }

    /// Emits save events for transactions completed by the shared store.
    pub fn poll_completed(&mut self, lua: &Lua, event_bus: &EventBus) {
        if self.pending_saves.is_empty() {
            return;
        }
        let completed = self.save_store.borrow_mut().take_completed();
        for (request_id, result) in completed {
            let Some(pending) = self.pending_saves.remove(&request_id) else {
                engine_core::omni_error!("Save store completed unknown request {request_id}");
                continue;
            };
            match result {
                Ok(()) => emit_save_succeeded(lua, event_bus, pending.lane, &pending.trigger),
                Err(error) => emit_save_failed(
                    lua, event_bus, pending.lane, &pending.trigger, &error.to_string(),
                ),
            }
        }
    }

    /// Records a pending request to load the latest runtime save.
    pub fn request_latest_runtime_load(&mut self) {
        self.pending_runtime_load_request = Some(RuntimeLoadRequest::Latest);
    }

    /// Returns the pending load request, if any.
    pub fn pending_runtime_load_request(&self) -> Option<RuntimeLoadRequest> {
        self.pending_runtime_load_request
    }

    /// Takes the pending load request, clearing it, and returns it.
    pub fn take_pending_runtime_load_request(&mut self) -> Option<RuntimeLoadRequest> {
        self.pending_runtime_load_request.take()
    }

    /// Shared reference to the provider registry.
    pub fn providers(&self) -> &Rc<RefCell<SaveProviderRegistry<'static>>> {
        &self.providers
    }

    /// Returns true if a committed latest save is available.
    pub fn has_latest_save(&self) -> bool {
        self.save_store.borrow().latest().ok().flatten().is_some()
    }

    /// Reads and validates the latest document for the authored game.
    pub fn load_latest_document(&self, game_id: Uuid) -> io::Result<Option<RuntimeSaveDocument>> {
        let document = self.save_store.borrow().latest()?;
        if let Some(document) = &document {
            let metadata = &document.metadata;
            if metadata.schema_version != RUNTIME_SAVE_SCHEMA_VERSION {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported save schema"));
            }
            if metadata.game_id != game_id {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "save belongs to another game"));
            }
            if metadata.slot != SaveSlotKey::Default {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported save slot"));
            }
        }
        Ok(document)
    }
}

fn current_unix_ms() -> u64 {
    // Clock-before-epoch is unreachable on modern hardware.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn coordinator_to_io_error(error: SaveCoordinatorError) -> io::Error {
    match error {
        SaveCoordinatorError::Capture { source, .. }
        | SaveCoordinatorError::Apply { source, .. } => source,
    }
}
