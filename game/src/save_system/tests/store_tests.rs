use super::RuntimeSaveTestContext;
use crate::save_system::{
    capture_document, runtime_latest_save_manifest_path, runtime_save_file, FileSaveStore,
    LatestRuntimeSaveManifest, RonPersist, RuntimeSaveDocument, RuntimeSaveMetadata, SaveLane,
    SaveProviderRegistry, SaveSlotKey, SaveStore, SaveStoreHandle, SaveWrite,
    RUNTIME_SAVE_SCHEMA_VERSION,
};
use engine_core::game::Game;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;

#[derive(Default)]
struct PendingStore {
    next_id: u64,
    pending: HashMap<u64, RuntimeSaveDocument>,
    completed: Vec<(u64, io::Result<()>)>,
    latest: Option<RuntimeSaveDocument>,
}

impl PendingStore {
    fn complete(&mut self, id: u64, result: io::Result<()>) -> io::Result<()> {
        let document = self.pending.remove(&id).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, format!("pending save request {id}"))
        })?;
        if result.is_ok() {
            self.latest = Some(document);
        }
        self.completed.push((id, result));
        Ok(())
    }
}

impl SaveStore for PendingStore {
    fn submit(&mut self, document: RuntimeSaveDocument) -> io::Result<SaveWrite> {
        self.next_id += 1;
        let id = self.next_id;
        self.pending.insert(id, document);
        Ok(SaveWrite::Pending(id))
    }

    fn take_completed(&mut self) -> Vec<(u64, io::Result<()>)> {
        std::mem::take(&mut self.completed)
    }

    fn latest(&self) -> io::Result<Option<RuntimeSaveDocument>> {
        Ok(self.latest.clone())
    }
}

fn document(game: &Game, lane: SaveLane) -> io::Result<RuntimeSaveDocument> {
    capture_document(
        &mut SaveProviderRegistry::new(),
        &RuntimeSaveMetadata {
            schema_version: RUNTIME_SAVE_SCHEMA_VERSION,
            game_id: game.id,
            game_name: game.name.clone(),
            slot: SaveSlotKey::Default,
            lane,
            saved_at_unix_ms: 123,
        },
    )
    .map_err(io::Error::other)
}

#[test]
fn pending_store_keeps_latest_at_last_committed_document() -> io::Result<()> {
    let game = Game::with_name("pending_store_game");
    let manual = document(&game, SaveLane::Manual)?;
    let autosave = document(&game, SaveLane::Autosave)?;
    assert_eq!(manual.metadata.game_id, game.id);
    assert_eq!(autosave.metadata.game_id, game.id);

    let mut store = PendingStore::default();
    let SaveWrite::Pending(first) = store.submit(manual.clone())? else {
        return Err(io::Error::other("expected pending manual save"));
    };
    assert_eq!(store.latest()?, None);
    assert!(store.take_completed().is_empty());

    store.complete(first, Ok(()))?;
    assert_eq!(store.latest()?, Some(manual.clone()));
    let results = store.take_completed();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, first);
    assert!(results[0].1.is_ok());

    let SaveWrite::Pending(second) = store.submit(autosave.clone())? else {
        return Err(io::Error::other("expected pending autosave"));
    };
    assert_eq!(store.latest()?, Some(manual.clone()));
    store.complete(second, Err(io::Error::other("transaction aborted")))?;
    assert_eq!(store.latest()?, Some(manual));
    let results = store.take_completed();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, second);
    assert!(results[0].1.is_err());

    let SaveWrite::Pending(third) = store.submit(autosave.clone())? else {
        return Err(io::Error::other("expected pending autosave"));
    };
    store.complete(third, Ok(()))?;
    assert_eq!(store.latest()?, Some(autosave));
    Ok(())
}

#[test]
fn file_store_preserves_native_document_and_latest_manifest_layout() -> io::Result<()> {
    let ctx = RuntimeSaveTestContext::new("file_store_native_layout");
    let game = Game::with_name(ctx.game_name());
    let manual = document(&game, SaveLane::Manual)?;
    let autosave = document(&game, SaveLane::Autosave)?;
    let handle: SaveStoreHandle = Rc::new(RefCell::new(FileSaveStore));

    assert_eq!(handle.borrow().latest()?, None);
    assert_eq!(handle.borrow_mut().submit(manual.clone())?, SaveWrite::Committed);
    assert_eq!(handle.borrow().latest()?, Some(manual.clone()));
    assert_eq!(
        RuntimeSaveDocument::read_from_path(&runtime_save_file(
            &SaveSlotKey::Default,
            SaveLane::Manual,
        ))?,
        manual,
    );
    assert!(handle.borrow_mut().take_completed().is_empty());

    assert_eq!(handle.borrow_mut().submit(autosave.clone())?, SaveWrite::Committed);
    assert_eq!(handle.borrow().latest()?, Some(autosave));
    let latest = LatestRuntimeSaveManifest::read_from_path(&runtime_latest_save_manifest_path())?;
    assert_eq!(latest.lane, SaveLane::Autosave);
    assert_eq!(latest.slot, SaveSlotKey::Default);
    assert_eq!(latest.game_name, game.name);
    Ok(())
}

#[test]
fn file_store_when_document_disagrees_with_latest_pointer_rejects_it() -> io::Result<()> {
    let ctx = RuntimeSaveTestContext::new("file_store_pointer_mismatch");
    let game = Game::with_name(ctx.game_name());
    let manual = document(&game, SaveLane::Manual)?;
    let mut store = FileSaveStore;
    store.submit(manual.clone())?;
    let path = runtime_save_file(&SaveSlotKey::Default, SaveLane::Manual);

    let mut wrong_lane = manual.clone();
    wrong_lane.metadata.lane = SaveLane::Autosave;
    let mut wrong_slot = manual;
    wrong_slot.metadata.slot = SaveSlotKey::Named("other".into());
    for invalid in [wrong_lane, wrong_slot] {
        invalid.write_to_path(&path)?;
        let error = store.latest().err().ok_or_else(|| io::Error::other("mismatch accepted"))?;
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
    Ok(())
}
