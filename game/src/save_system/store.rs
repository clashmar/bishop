use super::{
    runtime_latest_save_manifest_path, runtime_save_file, LatestRuntimeSaveManifest, RonPersist,
    RuntimeSaveDocument,
};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

/// The outcome of a submitted save; pending requests complete asynchronously.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveWrite {
    /// The document and latest pointer are durable.
    Committed,
    /// The request ID will receive a completion later.
    Pending(u64),
}

/// Shared save store used by the runtime and Lua contexts.
pub type SaveStoreHandle = Rc<RefCell<dyn SaveStore>>;

/// Persists a captured document and reports when its latest pointer is committed.
pub trait SaveStore {
    /// Submits a document and reports whether its latest pointer is committed.
    fn submit(&mut self, document: RuntimeSaveDocument) -> io::Result<SaveWrite>;
    /// Drains completed pending requests and their outcomes.
    fn take_completed(&mut self) -> Vec<(u64, io::Result<()>)>;
    /// Returns the document selected by the committed latest pointer.
    fn latest(&self) -> io::Result<Option<RuntimeSaveDocument>>;
}

/// Native store retaining the existing RON document and latest-manifest paths.
pub struct FileSaveStore;

impl SaveStore for FileSaveStore {
    fn submit(&mut self, document: RuntimeSaveDocument) -> io::Result<SaveWrite> {
        let metadata = &document.metadata;
        document.write_to_path(&runtime_save_file(&metadata.slot, metadata.lane))?;
        LatestRuntimeSaveManifest {
            lane: metadata.lane,
            slot: metadata.slot.clone(),
            game_name: metadata.game_name.clone(),
            saved_at_unix_ms: metadata.saved_at_unix_ms,
        }
        .write_to_path(&runtime_latest_save_manifest_path())?;
        Ok(SaveWrite::Committed)
    }

    fn take_completed(&mut self) -> Vec<(u64, io::Result<()>)> {
        Vec::new()
    }

    fn latest(&self) -> io::Result<Option<RuntimeSaveDocument>> {
        let manifest_path = runtime_latest_save_manifest_path();
        if !manifest_path.exists() {
            return Ok(None);
        }
        let manifest = LatestRuntimeSaveManifest::read_from_path(&manifest_path)?;
        let document = RuntimeSaveDocument::read_from_path(&runtime_save_file(
            &manifest.slot,
            manifest.lane,
        ))?;
        if document.metadata.slot != manifest.slot || document.metadata.lane != manifest.lane {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "latest save pointer disagrees with document metadata",
            ));
        }
        Ok(Some(document))
    }
}
