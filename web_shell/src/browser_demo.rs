//! Loads the authored Demo before driving the existing game Engine through Bishop WebGL2.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use bishop::BishopApp;
use bishop::input::{Input, KeyboardMouseState, MouseButton};
use bishop::prelude::PlatformContext;
use bishop::web::WebContext;
use engine_core::storage::{ResourceManifest, ResourceSource, load_game_shell_from_source};
use game_lib::engine::Engine;
use game_lib::startup::{assemble_browser_game, prepare_browser_game};

use crate::browser_fetch;
use web_shell::{DEMO_CHECKPOINT_ASSET_ROOT, demo_checkpoint_manifest_path};

type Downloaded = Result<(ResourceManifest, HashMap<String, Vec<u8>>), String>;

#[derive(Default)]
struct DemoState {
    pending: Option<Downloaded>,
    engine: Option<Engine>,
    failed: bool,
    ready: bool,
    frames: u64,
}

thread_local! {
    static DEMO: RefCell<DemoState> = RefCell::new(DemoState::default());
}

pub(crate) fn start() {
    let url = demo_checkpoint_manifest_path();
    let request_url = url.clone();
    browser_fetch::fetch(&request_url, move |result| {
        let manifest = result.and_then(|bytes| {
            let source = std::str::from_utf8(&bytes)
                .map_err(|error| format!("invalid UTF-8 in {url}: {error}"))?;
            let manifest: ResourceManifest = ron::from_str(source)
                .map_err(|error| format!("invalid resource manifest {url}: {error}"))?;
            manifest.validate().map_err(|error| format!("invalid {url}: {error}"))?;
            Ok(manifest)
        });
        match manifest {
            Ok(manifest) => fetch_next(Rc::new(manifest), 0, HashMap::new()),
            Err(error) => set_pending(Err(error)),
        }
    });
}

fn fetch_next(manifest: Rc<ResourceManifest>, index: usize, files: HashMap<String, Vec<u8>>) {
    let Some(entry) = manifest.files.get(index) else {
        set_pending(Ok(((*manifest).clone(), files)));
        return;
    };
    let path = entry.path.clone();
    let url = resource_url(&path);
    browser_fetch::fetch(&url, move |result| match result {
        Ok(bytes) => {
            let mut files = files;
            files.insert(path, bytes);
            fetch_next(manifest, index + 1, files);
        }
        Err(error) => set_pending(Err(format!("{path}: {error}"))),
    });
}

fn resource_url(relative: &str) -> String {
    let mut url = String::from(DEMO_CHECKPOINT_ASSET_ROOT);
    for byte in relative.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            url.push(char::from(byte));
        } else {
            url.push_str(&format!("%{byte:02X}"));
        }
    }
    url
}

fn set_pending(result: Downloaded) {
    DEMO.with(|state| state.borrow_mut().pending = Some(result));
}

fn fail(stage: &str, error: &str) {
    DEMO.with(|state| {
        let mut state = state.borrow_mut();
        state.failed = true;
        state.engine = None;
    });
    eprintln!("BISHOP_DEMO_ERROR {stage}: {error}");
}

fn assemble_downloaded(
    manifest: ResourceManifest,
    files: HashMap<String, Vec<u8>>,
) -> Result<Engine, String> {
    println!("BISHOP_DEMO_STAGE fetched_files={}", files.len());
    let source = ResourceSource::from_prefetched(manifest, files)
        .map_err(|error| format!("resource manifest: {error}"))?;
    println!("BISHOP_DEMO_STAGE source_validated");
    let game = load_game_shell_from_source(&source)
        .map_err(|error| format!("Demo split data: {error}"))?;
    println!("BISHOP_DEMO_STAGE shell_loaded name={}", game.name);
    let context = WebContext::new("#bishop-canvas")
        .map_err(|error| format!("Bishop WebGL2: {error}"))?;
    println!("BISHOP_DEMO_STAGE context_open");
    let ctx: PlatformContext = Rc::new(RefCell::new(context));
    let prepared = {
        let texture_loader = ctx.borrow();
        prepare_browser_game(game, source, &*texture_loader)
            .map_err(|error| format!("Demo startup: {error}"))?
    };
    println!("BISHOP_DEMO_STAGE game_prepared");
    let engine = assemble_browser_game(ctx, prepared)
        .map_err(|error| format!("Bishop Engine: {error}"))?;
    println!("BISHOP_DEMO_STAGE engine_assembled");
    Ok(engine)
}

pub(crate) fn with_input(f: impl FnOnce(&mut KeyboardMouseState)) {
    DEMO.with(|state| {
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

fn player_state(engine: &Engine) -> Result<String, String> {
    let game_ref = engine.game_instance.borrow();
    let game = &game_ref.game;
    let world = game.current_world();
    let room = world.current_room().ok_or_else(|| "no active authored Demo room".to_string())?;
    let player = game.ecs.get_player_entity().ok_or_else(|| "no authored Demo player".to_string())?;
    let position = game.ecs.get_player_transform()
        .ok_or_else(|| format!("Demo player {player:?} has no transform"))?.position;
    Ok(format!("game={} world={:?} room={:?} player={player:?} x={:.3} y={:.3}",
        game.name, world.id, room.id, position.x, position.y))
}

/// Called once per Emscripten animation frame; no HTTP request blocks the frame.
pub(crate) fn frame(_now_ms: f64) {
    let loaded = DEMO.with(|state| state.borrow_mut().pending.take());
    if let Some(loaded) = loaded {
        match loaded.and_then(|(manifest, files)| assemble_downloaded(manifest, files)) {
            Ok(engine) => DEMO.with(|state| state.borrow_mut().engine = Some(engine)),
            Err(error) => fail("load", &error),
        }
    }
    let result = DEMO.with(|state| {
        let mut state = state.borrow_mut();
        if state.failed {
            return Ok(());
        }
        let next_frame = state.frames + 1;
        let first_frame = !state.ready;
        let snapshot = {
            let Some(engine) = state.engine.as_mut() else { return Ok(()) };
            let ctx = engine.ctx.clone();
            ctx.borrow_mut().begin_frame()?;
            let input_position = ctx.borrow().any_key_pressed()
                .then(|| engine.game_instance.borrow().game.ecs.get_player_transform().map(|transform| transform.position))
                .flatten();
            if first_frame { println!("BISHOP_DEMO_STAGE first_frame_begin"); }
            pollster::block_on(engine.frame(ctx.clone()));
            if let Some(before) = input_position
                && let Some(after) = engine.game_instance.borrow().game.ecs.get_player_transform().map(|transform| transform.position)
            {
                println!("BISHOP_DEMO_INPUT_RESULT frame={next_frame} x={:.3}->{:.3} y={:.3}->{:.3}",
                    before.x, after.x, before.y, after.y);
            }
            ctx.borrow_mut().end_frame()?;
            ctx.borrow_mut().input_state().end_frame();
            if first_frame || next_frame % 60 == 0 {
                Some(player_state(engine)?)
            } else {
                None
            }
        };
        state.frames = next_frame;
        if let Some(snapshot) = snapshot {
            if first_frame {
                println!("BISHOP_DEMO_READY {snapshot}");
                state.ready = true;
            } else {
                println!("BISHOP_DEMO_STATE frame={next_frame} {snapshot}");
            }
        }
        Ok::<(), String>(())
    });
    if let Err(error) = result {
        fail("frame", &error);
    }
}
