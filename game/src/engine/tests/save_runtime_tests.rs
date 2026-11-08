use crate::engine::{EngineBuilder, GameInstance, RuntimeLoadRequest, SaveRuntime};
use crate::save_system::tests::RuntimeSaveTestContext;
use crate::save_system::{
    runtime_latest_save_manifest_path, runtime_save_file, FileSaveStore,
    LatestRuntimeSaveManifest, RestorePhase, RonPersist, RuntimeSaveDocument, RuntimeSaveMetadata,
    SaveLane, SaveProvider, SaveProviderId, SaveProviderRegistry, SaveSlotKey, SaveStore,
    SaveStoreHandle, SaveWrite, SavedSection, RUNTIME_SAVE_SCHEMA_VERSION,
};
use crate::scripting::lua_ctx::{register_save_lua_context, LuaSaveCtx};
use crate::scripting::modules::save_module::SaveModule;
use engine_core::game::Game;
use engine_core::scripting::lua_constants::{lua_engine, lua_events, lua_fields, lua_save};
use engine_core::scripting::LuaModule;
use engine_core::worlds::World;
use mlua::Lua;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::rc::Rc;
use uuid::Uuid;

struct StubSaveProvider;

type ControlledState = Rc<RefCell<ControlledStoreState>>;
type PendingDocuments = HashMap<u64, RuntimeSaveDocument>;
type CompletedWrites = Vec<(u64, io::Result<()>)>;
type SaveEventRecord = (String, String, String, Option<String>);
type RecordedEvents = Rc<RefCell<Vec<SaveEventRecord>>>;

#[derive(Default)]
struct ControlledStoreState {
    latest: Option<RuntimeSaveDocument>,
    submitted: Vec<RuntimeSaveDocument>,
    pending: PendingDocuments,
    completed: CompletedWrites,
    next_id: u64,
}

struct ControlledStore {
    state: ControlledState,
}

impl SaveProvider for StubSaveProvider {
    fn id(&self) -> SaveProviderId {
        SaveProviderId::new("game.flags")
    }

    fn restore_phase(&self) -> RestorePhase {
        RestorePhase::PostRuntime
    }

    fn capture(&mut self) -> io::Result<SavedSection> {
        Ok(SavedSection {
            version: 1,
            data: String::from("room=2"),
        })
    }

    fn apply(&mut self, _section: &SavedSection) -> io::Result<()> {
        Ok(())
    }
}

impl ControlledStoreState {
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

impl SaveStore for ControlledStore {
    fn submit(&mut self, document: RuntimeSaveDocument) -> io::Result<SaveWrite> {
        let mut state = self.state.borrow_mut();
        state.next_id += 1;
        let id = state.next_id;
        state.submitted.push(document.clone());
        state.pending.insert(id, document);
        Ok(SaveWrite::Pending(id))
    }

    fn take_completed(&mut self) -> CompletedWrites {
        std::mem::take(&mut self.state.borrow_mut().completed)
    }

    fn latest(&self) -> io::Result<Option<RuntimeSaveDocument>> {
        Ok(self.state.borrow().latest.clone())
    }
}

fn setup_save_runtime(
    prefix: &str,
) -> (
    RuntimeSaveTestContext,
    Rc<RefCell<GameInstance>>,
    SaveRuntime,
) {
    let ctx = RuntimeSaveTestContext::new(prefix);

    let save_providers = Rc::new(RefCell::new(SaveProviderRegistry::new()));
    save_providers
        .borrow_mut()
        .register(Box::new(StubSaveProvider))
        .unwrap();

    let game = Game::with_name(ctx.game_name());
    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    }));

    (
        ctx,
        game_instance,
        SaveRuntime::new(
            save_providers, Rc::new(Cell::new(false)), Rc::new(RefCell::new(FileSaveStore)),
        ),
    )
}

fn injected_store() -> (SaveStoreHandle, ControlledState) {
    let state = Rc::new(RefCell::new(ControlledStoreState::default()));
    let store: SaveStoreHandle = Rc::new(RefCell::new(ControlledStore { state: state.clone() }));
    (store, state)
}

fn save_document(game: &Game) -> RuntimeSaveDocument {
    RuntimeSaveDocument {
        metadata: RuntimeSaveMetadata {
            schema_version: RUNTIME_SAVE_SCHEMA_VERSION,
            game_id: game.id,
            game_name: game.name.clone(),
            slot: SaveSlotKey::Default,
            lane: SaveLane::Manual,
            saved_at_unix_ms: 123,
        },
        sections: HashMap::new(),
    }
}

#[test]
fn save_to_lane_writes_document_and_latest_manifest() {
    let (_ctx, game_instance, mut save_runtime) = setup_save_runtime("save_runtime_manual");

    save_runtime
        .save_to_lane(&game_instance, SaveLane::Manual)
        .unwrap();

    let save_path = runtime_save_file(&SaveSlotKey::Default, SaveLane::Manual);
    let manifest_path = runtime_latest_save_manifest_path();
    assert!(save_path.exists());
    assert!(manifest_path.exists());
}

#[test]
fn request_latest_runtime_load_sets_pending_request() {
    let (_ctx, _game_instance, mut save_runtime) = setup_save_runtime("save_runtime_latest");

    save_runtime.request_latest_runtime_load();

    assert_eq!(
        save_runtime.pending_runtime_load_request(),
        Some(RuntimeLoadRequest::Latest)
    );
}

#[test]
fn take_pending_runtime_load_request_clears_request() {
    let (_ctx, _game_instance, mut save_runtime) = setup_save_runtime("save_runtime_take");

    save_runtime.request_latest_runtime_load();

    let taken = save_runtime.take_pending_runtime_load_request();
    assert_eq!(taken, Some(RuntimeLoadRequest::Latest));

    assert_eq!(save_runtime.pending_runtime_load_request(), None);
}

#[test]
fn load_latest_document_reads_manifest_target() {
    let (_ctx, game_instance, mut save_runtime) =
        setup_save_runtime("save_runtime_load_latest");

    save_runtime
        .save_to_lane(&game_instance, SaveLane::Manual)
        .unwrap();

    let document = save_runtime
        .load_latest_document(game_instance.borrow().game.id)
        .unwrap()
        .expect("expected a saved document");

    assert_eq!(document.metadata.lane, SaveLane::Manual);
    assert!(document.sections.contains_key("game.flags"));
    assert_eq!(document.sections.len(), 1);
}

#[test]
fn load_latest_document_errors_on_manifest_with_missing_target() {
    let ctx = RuntimeSaveTestContext::new("save_runtime_bad_manifest");

    // Write a manifest that points to a lane that has no save file
    let manifest = LatestRuntimeSaveManifest {
        lane: SaveLane::Autosave,
        slot: SaveSlotKey::Default,
        game_name: ctx.game_name().to_string(),
        saved_at_unix_ms: 42,
    };
    manifest
        .write_to_path(&runtime_latest_save_manifest_path())
        .unwrap();

    // The save file does not exist, so load_latest_document should error
    let save_providers = Rc::new(RefCell::new(SaveProviderRegistry::new()));
    let save_runtime = SaveRuntime::new(
        save_providers, Rc::new(Cell::new(false)), Rc::new(RefCell::new(FileSaveStore)),
    );
    let result = save_runtime.load_latest_document(Uuid::nil());

    assert!(
        result.is_err(),
        "expected error for manifest with missing target"
    );
}

#[test]
fn save_triggered_from_command_queue_writes_document_and_manifest() {
    let ctx = RuntimeSaveTestContext::new("save_runtime_cmd_queue");

    // Set up Lua with save context and module
    let lua = Lua::new();
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().unwrap())
        .unwrap();

    let save_providers = Rc::new(RefCell::new(SaveProviderRegistry::new()));
    register_save_lua_context(
        &lua, save_providers.clone(), Rc::new(RefCell::new(FileSaveStore)),
        Rc::new(Cell::new(false)),
    ).unwrap();
    SaveModule.register(&lua).unwrap();

    // Register a trivial Lua save provider so capture succeeds
    lua.load(format!(
        r#"engine.{}.{}{{["{}"]="game.test",["{}"]=1,["{}"]=function() return "{{"..[["room"]]..":2}}" end,["{}"]=function(_) end}}"#,
        lua_save::SAVE, lua_save::REGISTER_PROVIDER,
        lua_fields::ID, lua_save::PROVIDER_VERSION,
        lua_save::PROVIDER_CAPTURE, lua_save::PROVIDER_APPLY,
    )).exec().unwrap();

    let mut game = Game::with_name(ctx.game_name());
    game.add_world(World::default());
    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(), traversal_residency_diagnostics: None,
    }));

    let mut save_runtime = SaveRuntime::new(
        save_providers, Rc::new(Cell::new(false)), Rc::new(RefCell::new(FileSaveStore)),
    );
    save_runtime
        .save_to_lane(&game_instance, SaveLane::Manual)
        .unwrap();

    let save_path = runtime_save_file(&SaveSlotKey::Default, SaveLane::Manual);
    let manifest_path = runtime_latest_save_manifest_path();
    assert!(
        save_path.exists(),
        "save file should exist at {:?}",
        save_path
    );
    assert!(
        manifest_path.exists(),
        "manifest should exist at {:?}",
        manifest_path
    );
}

#[test]
fn quit_to_title_flag_is_shared_between_lua_and_save_runtime() {
    let lua = Lua::new();
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().unwrap())
        .unwrap();

    let flag = Rc::new(Cell::new(false));
    let save_providers = Rc::new(RefCell::new(SaveProviderRegistry::new()));
    register_save_lua_context(
        &lua, save_providers.clone(), Rc::new(RefCell::new(FileSaveStore)), flag.clone(),
    ).unwrap();

    let save_runtime = SaveRuntime::new(
        save_providers, flag.clone(), Rc::new(RefCell::new(FileSaveStore)),
    );

    assert!(!save_runtime.pending_quit_to_title.get());

    // Simulate Lua calling engine.quit_to_title()
    LuaSaveCtx::borrow_ctx(&lua)
        .unwrap()
        .pending_quit_to_title
        .set(true);

    // Both should see the change
    assert!(save_runtime.pending_quit_to_title.get());
    assert!(flag.get());
}

#[test]
fn save_runtime_tests_when_store_is_injected_shares_latest_with_lua() -> mlua::Result<()> {
    let game = Game::with_name("injected_store_game");
    let document = save_document(&game);
    let (store, state) = injected_store();
    state.borrow_mut().latest = Some(document.clone());
    let builder = EngineBuilder::new().save_store(store.clone());
    builder.lua.globals().set(lua_engine::ENGINE, builder.lua.create_table()?)?;
    builder.register_save_context()?;
    SaveModule.register(&builder.lua)?;

    let has_latest: bool = builder.lua.load(format!(
        "return engine.{}.{}()", lua_save::SAVE, lua_save::HAS_LATEST,
    )).eval()?;
    assert!(has_latest);
    assert!(Rc::ptr_eq(&LuaSaveCtx::borrow_ctx(&builder.lua)?.save_store, &store));

    let runtime = SaveRuntime::new(
        builder.save_providers.clone(), builder.pending_quit_to_title.clone(),
        builder.save_store.clone(),
    );
    assert_eq!(runtime.load_latest_document(game.id).map_err(mlua::Error::external)?, Some(document));
    Ok(())
}

#[test]
fn save_runtime_tests_when_storage_unavailable_keeps_latest_hidden() -> mlua::Result<()> {
    let (store, _state) = injected_store();
    let builder = EngineBuilder::new().save_store(store);
    builder.lua.globals().set(lua_engine::ENGINE, builder.lua.create_table()?)?;
    builder.register_save_context()?;
    SaveModule.register(&builder.lua)?;

    let has_latest: bool = builder.lua.load(format!(
        "return engine.{}.{}()", lua_save::SAVE, lua_save::HAS_LATEST,
    )).eval()?;
    assert!(!has_latest);
    Ok(())
}

#[test]
fn save_runtime_tests_when_save_metadata_is_invalid_rejects_before_restore() {
    let game = Game::with_name("validation_test_game");
    let (store, state) = injected_store();
    let runtime = SaveRuntime::new(
        Rc::new(RefCell::new(SaveProviderRegistry::new())),
        Rc::new(Cell::new(false)), store,
    );
    let mut wrong_game = save_document(&game);
    wrong_game.metadata.game_id = Uuid::new_v4();
    let mut wrong_schema = save_document(&game);
    wrong_schema.metadata.schema_version += 1;
    let mut wrong_slot = save_document(&game);
    wrong_slot.metadata.slot = SaveSlotKey::Named("other".to_string());

    for invalid in [wrong_game, wrong_schema, wrong_slot] {
        state.borrow_mut().latest = Some(invalid);
        let error = runtime.load_latest_document(game.id).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}

#[test]
fn save_event_waits_for_store_completion() -> mlua::Result<()> {
    let lua = Lua::new();
    let game = Game::with_name("pending_save_events");
    let game_id = game.id;
    let event_bus = game.script_manager.event_bus.clone();
    let events: RecordedEvents = Rc::new(RefCell::new(Vec::new()));
    let succeeded = events.clone();
    event_bus.on(
        lua_events::SAVE_SUCCEEDED.to_string(),
        lua.create_function(move |_, (lane, trigger): (String, String)| {
            succeeded.borrow_mut().push((lua_events::SAVE_SUCCEEDED.to_string(), lane, trigger, None));
            Ok(())
        })?,
    );
    let failed = events.clone();
    event_bus.on(
        lua_events::SAVE_FAILED.to_string(),
        lua.create_function(move |_, (lane, trigger, error): (String, String, String)| {
            failed.borrow_mut().push((lua_events::SAVE_FAILED.to_string(), lane, trigger, Some(error)));
            Ok(())
        })?,
    );
    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    }));
    let providers = Rc::new(RefCell::new(SaveProviderRegistry::new()));
    providers.borrow_mut().register(Box::new(StubSaveProvider))
        .map_err(mlua::Error::external)?;
    let (store, state) = injected_store();
    let mut runtime = SaveRuntime::new(providers, Rc::new(Cell::new(false)), store);

    let SaveWrite::Pending(manual_id) = runtime.save_to_lane(&game_instance, SaveLane::Manual)
        .map_err(mlua::Error::external)? else {
        return Err(mlua::Error::RuntimeError("manual save did not pend".into()));
    };
    runtime.track_pending(manual_id, SaveLane::Manual, lua_save::MANUAL.to_string());
    runtime.poll_completed(&lua, &event_bus);
    assert!(events.borrow().is_empty());
    assert!(runtime.load_latest_document(game_id).map_err(mlua::Error::external)?.is_none());
    assert_eq!(state.borrow().submitted.len(), 1);
    state.borrow_mut().complete(manual_id, Ok(())).map_err(mlua::Error::external)?;
    runtime.poll_completed(&lua, &event_bus);
    assert_eq!(*events.borrow(), vec![(
        lua_events::SAVE_SUCCEEDED.to_string(), SaveLane::Manual.file_stem().to_string(),
        lua_save::MANUAL.to_string(), None,
    )]);
    assert_eq!(runtime.load_latest_document(game_id).map_err(mlua::Error::external)?
        .map(|document| document.metadata.lane), Some(SaveLane::Manual));

    let SaveWrite::Pending(autosave_id) = runtime.save_to_lane(&game_instance, SaveLane::Autosave)
        .map_err(mlua::Error::external)? else {
        return Err(mlua::Error::RuntimeError("autosave did not pend".into()));
    };
    runtime.track_pending(autosave_id, SaveLane::Autosave, lua_save::AUTO.to_string());
    runtime.poll_completed(&lua, &event_bus);
    assert_eq!(events.borrow().len(), 1);
    state.borrow_mut().complete(autosave_id, Err(io::Error::other("transaction aborted")))
        .map_err(mlua::Error::external)?;
    runtime.poll_completed(&lua, &event_bus);
    assert_eq!(events.borrow().len(), 2);
    {
        let recorded = events.borrow();
        let failed_event = &recorded[1];
        assert_eq!(failed_event.0, lua_events::SAVE_FAILED);
        assert_eq!(failed_event.1, SaveLane::Autosave.file_stem());
        assert_eq!(failed_event.2, lua_save::AUTO);
        assert!(failed_event.3.as_deref().is_some_and(|error| error.contains("transaction aborted")));
    }
    assert_eq!(runtime.load_latest_document(game_id).map_err(mlua::Error::external)?
        .map(|document| document.metadata.lane), Some(SaveLane::Manual));
    runtime.poll_completed(&lua, &event_bus);
    assert_eq!(events.borrow().len(), 2);

    let SaveWrite::Pending(next_id) = runtime.save_to_lane(&game_instance, SaveLane::Autosave)
        .map_err(mlua::Error::external)? else {
        return Err(mlua::Error::RuntimeError("second autosave did not pend".into()));
    };
    runtime.track_pending(next_id, SaveLane::Autosave, lua_save::AUTO.to_string());
    state.borrow_mut().complete(next_id, Ok(())).map_err(mlua::Error::external)?;
    runtime.poll_completed(&lua, &event_bus);
    assert_eq!(events.borrow().len(), 3);
    assert_eq!(events.borrow()[2].0, lua_events::SAVE_SUCCEEDED);
    assert_eq!(runtime.load_latest_document(game_id).map_err(mlua::Error::external)?
        .map(|document| document.metadata.lane), Some(SaveLane::Autosave));
    Ok(())
}
