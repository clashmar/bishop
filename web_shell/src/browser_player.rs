//! Game-independent browser entry for a URL-selected, versioned authored bundle.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, c_char};
use std::io;
use std::rc::Rc;

use bishop::BishopApp;
use bishop::input::{KeyboardMouseState, MouseButton};
use bishop::prelude::PlatformContext;
use bishop::web::WebContext;
use engine_core::game::Game;
use engine_core::storage::web_game_bundle::WebGameBundle;
use engine_core::storage::{ResourceKey, ResourceSource, load_game_shell_from_source};
use game_lib::engine::{Engine, RuntimeLoadRequest};
use game_lib::save_system::{BrowserSaveStore, OpenedBrowserStore, SaveStoreHandle};
use game_lib::startup::{assemble_browser_game, prepare_browser_game, restore_browser_game};

use crate::browser_fetch;
use crate::player_bundle::{DownloadPlan, parse_manifest, resource_url};

type Downloaded = Result<(WebGameBundle, HashMap<ResourceKey, Vec<u8>>), String>;

struct OpeningGame {
    game: Game,
    source: ResourceSource,
}

struct ActiveGame {
    source: ResourceSource,
    save_store: SaveStoreHandle,
}

#[derive(Default)]
struct PlayerState {
    pending: Option<Downloaded>,
    opening: Option<OpeningGame>,
    active: Option<ActiveGame>,
    engine: Option<Engine>,
    failed: bool,
    ready: bool,
}

thread_local! {
    static PLAYER: RefCell<PlayerState> = RefCell::new(PlayerState::default());
}

unsafe extern "C" {
    fn emscripten_run_script_string(script: *const c_char) -> *const c_char;
}

fn selected_manifest_url() -> Result<String, String> {
    let script = c"window.bishopGameManifestUrl || ''";
    let pointer = unsafe { emscripten_run_script_string(script.as_ptr()) };
    if pointer.is_null() {
        return Err("no selected game manifest URL".into());
    }
    let url = unsafe { CStr::from_ptr(pointer) }.to_str()
        .map_err(|error| format!("invalid game manifest URL: {error}"))?;
    if url.is_empty() {
        return Err("no selected game manifest URL".into());
    }
    Ok(url.to_owned())
}

pub(crate) fn start() {
    let manifest_url = match selected_manifest_url() {
        Ok(url) => url,
        Err(error) => { fail("selection", &error); return; }
    };
    println!("BISHOP_PLAYER_STAGE manifest_request {manifest_url}");
    let request_url = manifest_url.clone();
    browser_fetch::fetch(&request_url, move |result| {
        match result.and_then(|bytes| parse_manifest(&bytes)) {
            Ok(bundle) => {
                println!("BISHOP_PLAYER_STAGE manifest_validated files={}", bundle.resources.files.len());
                fetch_resources(manifest_url, bundle);
            }
            Err(error) => set_pending(Err(error)),
        }
    });
}

struct DownloadQueue {
    manifest_url: String,
    bundle: WebGameBundle,
    plan: DownloadPlan,
    files: HashMap<ResourceKey, Vec<u8>>,
}

fn fetch_resources(manifest_url: String, bundle: WebGameBundle) {
    let lengths = bundle.resources.files.iter().map(|entry| entry.byte_len).collect();
    pump(Rc::new(RefCell::new(DownloadQueue {
        manifest_url, bundle, plan: DownloadPlan::new(lengths), files: HashMap::new(),
    })));
}

fn pump(queue: Rc<RefCell<DownloadQueue>>) {
    loop {
        let next = {
            let mut queue = queue.borrow_mut();
            queue.plan.next().map(|index| {
                (index, queue.bundle.resources.files[index].path.clone(), queue.manifest_url.clone())
            })
        };
        let Some((index, path, manifest_url)) = next else { return };
        let url = match resource_url(&manifest_url, &path) {
            Ok(url) => url,
            Err(error) => {
                queue.borrow_mut().plan.fail();
                set_pending(Err(error));
                return;
            }
        };
        let request = queue.clone();
        browser_fetch::fetch(&url, move |result| resource_finished(request, index, path, result));
    }
}

fn resource_finished(
    queue: Rc<RefCell<DownloadQueue>>,
    index: usize,
    path: ResourceKey,
    result: Result<Vec<u8>, String>,
) {
    let outcome = {
        let mut queue = queue.borrow_mut();
        if queue.plan.is_failed() { return; }
        match result {
            Err(error) => {
                queue.plan.fail();
                Err(format!("{path}: {error}"))
            }
            Ok(bytes) => match queue.plan.finish(index, bytes.len()) {
                Err(error) => Err(format!("{path}: {error}")),
                Ok(progress) => {
                    queue.files.insert(path, bytes);
                    let complete = progress.is_complete().then(|| {
                        (queue.bundle.clone(), std::mem::take(&mut queue.files))
                    });
                    Ok((progress, complete))
                }
            },
        }
    };
    match outcome {
        Err(error) => set_pending(Err(error)),
        Ok((progress, complete)) => {
            println!("BISHOP_PLAYER_PROGRESS files={}/{} bytes={}/{}",
                progress.completed_files, progress.total_files,
                progress.loaded_bytes, progress.total_bytes);
            if let Some(downloaded) = complete {
                set_pending(Ok(downloaded));
            } else {
                pump(queue);
            }
        }
    }
}

fn set_pending(result: Downloaded) {
    PLAYER.with(|state| state.borrow_mut().pending = Some(result));
}

fn fail(stage: &str, error: &str) {
    PLAYER.with(|state| {
        let mut state = state.borrow_mut();
        state.failed = true;
        state.engine = None;
        state.active = None;
    });
    eprintln!("BISHOP_PLAYER_ERROR {stage}: {error}");
}

fn prepare_downloaded(
    bundle: WebGameBundle,
    files: HashMap<ResourceKey, Vec<u8>>,
) -> Result<OpeningGame, String> {
    let source = ResourceSource::from_prefetched(bundle.resources, files)
        .map_err(|error| format!("game resources: {error}"))?;
    let game = load_game_shell_from_source(&source)
        .map_err(|error| format!("authored game data: {error}"))?;
    println!("BISHOP_PLAYER_STAGE shell_loaded name={}", game.name);
    Ok(OpeningGame { game, source })
}

fn assemble_opened(opening: OpeningGame, store: BrowserSaveStore) -> Result<(Engine, ActiveGame), String> {
    let OpeningGame { game, source } = opening;
    let context = WebContext::new("#bishop-canvas")
        .map_err(|error| format!("Bishop WebGL2: {error}"))?;
    let ctx: PlatformContext = Rc::new(RefCell::new(context));
    let save_store: SaveStoreHandle = Rc::new(RefCell::new(store));
    let active = ActiveGame { source: source.clone(), save_store: save_store.clone() };
    let prepared = {
        let texture_loader = ctx.borrow();
        prepare_browser_game(game, source, &*texture_loader, save_store)
            .map_err(|error| format!("authored startup: {error}"))?
    };
    let engine = assemble_browser_game(ctx, prepared)
        .map_err(|error| format!("Bishop Engine: {error}"))?;
    println!("BISHOP_PLAYER_STAGE engine_assembled");
    Ok((engine, active))
}

fn complete_open(opening: OpeningGame, result: io::Result<OpenedBrowserStore>) {
    let game_id = opening.game.id;
    let store = match result.and_then(|opened| BrowserSaveStore::from_open(game_id, opened)) {
        Ok(store) => store,
        Err(error) => BrowserSaveStore::unavailable(game_id, error.to_string()),
    };
    if let Some(reason) = store.unavailable_reason() {
        eprintln!("BISHOP_PLAYER_SAVE_UNAVAILABLE {reason}");
    }
    match assemble_opened(opening, store) {
        Ok((engine, active)) => PLAYER.with(|state| {
            let mut state = state.borrow_mut();
            state.engine = Some(engine);
            state.active = Some(active);
        }),
        Err(error) => fail("load", &error),
    }
}

pub(crate) fn with_input(f: impl FnOnce(&mut KeyboardMouseState)) {
    PLAYER.with(|state| {
        let state = state.borrow();
        if let Some(engine) = state.engine.as_ref() {
            f(engine.ctx.borrow_mut().input_state());
        }
    });
}

pub(crate) fn mouse_down(button: MouseButton, x: f32, y: f32, time_ms: f64) {
    with_input(|input| {
        input.on_mouse_move(x, y);
        input.on_mouse_down(button, time_ms / 1000.0, (x, y));
    });
}

pub(crate) fn focus_changed(active: bool) {
    if !active {
        with_input(KeyboardMouseState::clear_on_focus_loss);
    }
}

/// Called once per animation frame without blocking an HTTP request.
pub(crate) fn frame(_now_ms: f64) {
    let loaded = PLAYER.with(|state| state.borrow_mut().pending.take());
    if let Some(loaded) = loaded {
        match loaded.and_then(|(bundle, files)| prepare_downloaded(bundle, files)) {
            Ok(opening) => match BrowserSaveStore::begin_open(opening.game.id) {
                Ok(()) => PLAYER.with(|state| state.borrow_mut().opening = Some(opening)),
                Err(error) => complete_open(opening, Err(error)),
            },
            Err(error) => fail("load", &error),
        }
    }
    if PLAYER.with(|state| state.borrow().opening.is_some()) {
        let opened = match BrowserSaveStore::poll_open() {
            Ok(None) => None,
            Ok(Some(opened)) => Some(Ok(opened)),
            Err(error) => Some(Err(error)),
        };
        if let Some(result) = opened {
            if let Some(opening) = PLAYER.with(|state| state.borrow_mut().opening.take()) {
                complete_open(opening, result);
            }
        }
    }
    let result = PLAYER.with(|state| {
        let mut state = state.borrow_mut();
        if state.failed {
            return Ok(None);
        }
        let Some(engine) = state.engine.as_mut() else { return Ok(None) };
        let ctx = engine.ctx.clone();
        ctx.borrow_mut().begin_frame()?;
        pollster::block_on(engine.frame(ctx.clone()));
        ctx.borrow_mut().end_frame()?;
        ctx.borrow_mut().input_state().end_frame();
        let request = engine.save_runtime.take_pending_runtime_load_request();
        if !state.ready {
            println!("BISHOP_PLAYER_READY");
            state.ready = true;
        }
        Ok::<_, String>(request)
    });
    match result {
        Ok(Some(RuntimeLoadRequest::Latest)) => restore_latest(),
        Ok(None) => {}
        Err(error) => fail("frame", &error),
    }
}

fn restore_latest() {
    let retained = PLAYER.with(|state| {
        let state = state.borrow();
        let active = state.active.as_ref().ok_or("browser game lost its authored resource source")?;
        let ctx = state.engine.as_ref().ok_or("browser game lost its running engine")?.ctx.clone();
        Ok::<_, &str>((active.source.clone(), active.save_store.clone(), ctx))
    });
    let result = retained.map_err(str::to_owned)
        .and_then(|(source, store, ctx)| restore_browser_game(source, ctx, store));
    match result {
        Ok(engine) => PLAYER.with(|state| state.borrow_mut().engine = Some(engine)),
        Err(error) => eprintln!("BISHOP_PLAYER_LOAD_ERROR {error}"),
    }
}
