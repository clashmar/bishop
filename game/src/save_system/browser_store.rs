use super::{
    RonPersist, RuntimeSaveDocument, SaveSlotKey, SaveStore, SaveWrite,
    RUNTIME_SAVE_SCHEMA_VERSION,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_int};
use std::io;
use uuid::Uuid;

unsafe extern "C" {
    fn emscripten_run_script_int(script: *const c_char) -> c_int;
    fn emscripten_run_script_string(script: *const c_char) -> *const c_char;
}

/// Completed browser storage open, including its latest saved document.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedBrowserStore {
    status: String,
    latest: Option<String>,
    slot: Option<String>,
    lane: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserWriteResult {
    request_id: u64,
    ok: bool,
    error: Option<String>,
}

/// IndexedDB-backed runtime save store for one authored game.
pub struct BrowserSaveStore {
    game_id: Uuid,
    latest: Option<RuntimeSaveDocument>,
    unavailable: Option<String>,
    pending: HashMap<u64, RuntimeSaveDocument>,
    next_request_id: u64,
}

impl BrowserSaveStore {
    /// Starts opening the selected game's IndexedDB save in the browser.
    pub fn begin_open(game_id: Uuid) -> io::Result<()> {
        let id = serde_json::to_string(&game_id.to_string()).map_err(io::Error::other)?;
        run_script_int(&format!("window.bishopSaveStore.beginOpen({id}); 1"))
    }

    /// Takes an available browser open result without blocking the frame.
    pub fn poll_open() -> io::Result<Option<OpenedBrowserStore>> {
        let result = run_script_string("JSON.stringify(window.bishopSaveStore.takeOpenResult())")?;
        serde_json::from_str(&result).map_err(invalid_data)
    }

    /// Creates a store using the browser's completed open result.
    pub fn from_open(game_id: Uuid, opened: OpenedBrowserStore) -> io::Result<Self> {
        if opened.status == "unavailable" {
            return Ok(Self::unavailable(
                game_id,
                opened.error.unwrap_or_else(|| "browser save storage is unavailable".into()),
            ));
        }
        if opened.status != "ready" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid browser save open result"));
        }
        let latest = match opened.latest {
            None => {
                if opened.slot.is_some() || opened.lane.is_some() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "save pointer has no document"));
                }
                None
            }
            Some(ron) => {
                let document = RuntimeSaveDocument::from_ron_str(&ron).map_err(invalid_data)?;
                let metadata = &document.metadata;
                if metadata.schema_version != RUNTIME_SAVE_SCHEMA_VERSION
                    || metadata.game_id != game_id
                    || metadata.slot != SaveSlotKey::Default
                    || opened.slot.as_deref() != Some(metadata.slot.folder_name().as_str())
                    || opened.lane.as_deref() != Some(metadata.lane.file_stem())
                {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid browser save metadata"));
                }
                Some(document)
            }
        };
        Ok(Self { game_id, latest, unavailable: None, pending: HashMap::new(), next_request_id: 0 })
    }

    /// Returns why browser storage cannot be used, if unavailable.
    pub fn unavailable_reason(&self) -> Option<&str> {
        self.unavailable.as_deref()
    }

    /// Disables loading while allowing gameplay to continue without storage.
    pub fn unavailable(game_id: Uuid, reason: impl Into<String>) -> Self {
        Self {
            game_id, latest: None, unavailable: Some(reason.into()),
            pending: HashMap::new(), next_request_id: 0,
        }
    }
}

impl SaveStore for BrowserSaveStore {
    fn submit(&mut self, document: RuntimeSaveDocument) -> io::Result<SaveWrite> {
        if let Some(error) = &self.unavailable {
            return Err(io::Error::other(error.clone()));
        }
        if document.metadata.game_id != self.game_id {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "save belongs to another game"));
        }
        let ron = document.to_ron_string()?;
        let id = serde_json::to_string(&self.game_id.to_string()).map_err(io::Error::other)?;
        let slot = serde_json::to_string(&document.metadata.slot.folder_name()).map_err(io::Error::other)?;
        let lane = serde_json::to_string(document.metadata.lane.file_stem()).map_err(io::Error::other)?;
        let ron = serde_json::to_string(&ron).map_err(io::Error::other)?;
        let request_id = self.next_request_id + 1;
        run_script_int(&format!(
            "window.bishopSaveStore.write({id}, {slot}, {lane}, {ron}, {request_id}).catch(() => {{}}); 1"
        ))?;
        self.next_request_id = request_id;
        self.pending.insert(request_id, document);
        Ok(SaveWrite::Pending(request_id))
    }

    fn take_completed(&mut self) -> Vec<(u64, io::Result<()>)> {
        let mut completions = Vec::new();
        loop {
            let raw = match run_script_string("JSON.stringify(window.bishopSaveStore.takeResult())") {
                Ok(raw) => raw,
                Err(error) => {
                    for (id, _) in self.pending.drain() {
                        completions.push((id, Err(io::Error::other(error.to_string()))));
                    }
                    return completions;
                }
            };
            let result: Option<BrowserWriteResult> = match serde_json::from_str(&raw) {
                Ok(result) => result,
                Err(error) => {
                    for (id, _) in self.pending.drain() {
                        completions.push((id, Err(invalid_data(error.to_string()))));
                    }
                    return completions;
                }
            };
            let Some(result) = result else { break };
            let Some(document) = self.pending.remove(&result.request_id) else { continue };
            if result.ok {
                self.latest = Some(document);
                completions.push((result.request_id, Ok(())));
            } else {
                completions.push((result.request_id, Err(io::Error::other(
                    result.error.unwrap_or_else(|| "browser save transaction failed".into()),
                ))));
            }
        }
        completions
    }

    fn latest(&self) -> io::Result<Option<RuntimeSaveDocument>> {
        Ok(self.latest.clone())
    }
}

fn run_script_int(script: &str) -> io::Result<()> {
    let script = CString::new(script).map_err(invalid_data)?;
    let result = unsafe { emscripten_run_script_int(script.as_ptr()) };
    if result == 1 { Ok(()) } else { Err(io::Error::other("browser save bridge failed")) }
}

fn run_script_string(script: &str) -> io::Result<String> {
    let script = CString::new(script).map_err(invalid_data)?;
    let pointer = unsafe { emscripten_run_script_string(script.as_ptr()) };
    if pointer.is_null() {
        return Err(io::Error::other("browser save bridge returned no result"));
    }
    unsafe { CStr::from_ptr(pointer) }.to_str().map(str::to_owned).map_err(invalid_data)
}

fn invalid_data(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}
