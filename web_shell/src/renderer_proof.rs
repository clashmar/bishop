//! Separate authored-frame proof: fetch Demo assets, draw entirely through Bishop WebGL2.

use std::cell::RefCell;
use std::collections::BTreeMap;

use bishop::camera::{Camera, Camera2D};
use bishop::draw::{Draw, DrawTextureParams};
use bishop::material::RenderOps;
use bishop::text::{Text, TextParams};
use bishop::texture::TextureLoader;
use bishop::types::{Color, Rect, Vec2};
use bishop::web::WebContext;
use serde::Deserialize;

use crate::browser_fetch;
use web_shell::{DEMO_RENDERER_SHEET_PATH, demo_renderer_atlas_path};

#[derive(Deserialize)]
struct Atlas {
    frames: BTreeMap<String, FrameEntry>,
}

#[derive(Deserialize)]
struct FrameEntry {
    frame: FrameRect,
}

#[derive(Clone, Copy, Deserialize)]
struct FrameRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

type LoadedAssets = Result<(Vec<u8>, Vec<u8>), String>;

#[derive(Default)]
struct ProofState {
    pending: Option<LoadedAssets>,
    context: Option<WebContext>,
}

thread_local! {
    static PROOF: RefCell<ProofState> = RefCell::new(ProofState::default());
}

pub(crate) fn start() {
    browser_fetch::fetch(DEMO_RENDERER_SHEET_PATH, |png| match png {
        Err(error) => set_pending(Err(error)),
        Ok(png) => {
            let atlas_url = demo_renderer_atlas_path();
            browser_fetch::fetch(&atlas_url, move |atlas| {
                set_pending(atlas.map(|atlas| (png, atlas)));
            });
        }
    });
}

fn set_pending(result: LoadedAssets) {
    PROOF.with(|state| state.borrow_mut().pending = Some(result));
}

/// Called by the Emscripten browser loop, not by an HTML/Canvas renderer.
pub(crate) fn frame(_now_ms: f64) {
    let loaded = PROOF.with(|state| {
        let mut state = state.borrow_mut();
        if state.context.is_some() {
            None
        } else {
            state.pending.take()
        }
    });
    let Some(loaded) = loaded else { return };
    match loaded.and_then(|(png, atlas)| draw_authored_frame(&png, &atlas)) {
        Ok((context, frame, text_width)) => {
            PROOF.with(|state| state.borrow_mut().context = Some(context));
            println!("BISHOP_WEBGL_RENDER_OK frame={}x{} text={text_width}", frame.w, frame.h);
        }
        Err(error) => eprintln!("BISHOP_WEBGL_RENDER_ERROR {error}"),
    }
}

fn draw_authored_frame(png: &[u8], atlas_bytes: &[u8]) -> Result<(WebContext, FrameRect, f32), String> {
    let atlas: Atlas = serde_json::from_slice(atlas_bytes).map_err(|error| error.to_string())?;
    let (frame_name, entry) = atlas.frames.into_iter().next()
        .ok_or_else(|| "Demo Idle atlas contains no frames".to_owned())?;
    let frame = entry.frame;
    let mut context = WebContext::new("#bishop-canvas")?;
    let sheet = context.load_texture_from_bytes(png)?;
    if frame.w == 0 || frame.h == 0
        || !frame.x.checked_add(frame.w).is_some_and(|right| right as f32 <= sheet.width())
        || !frame.y.checked_add(frame.h).is_some_and(|bottom| bottom as f32 <= sheet.height())
    {
        return Err(format!("Demo atlas frame {frame_name:?} is outside its decoded PNG"));
    }

    context.begin_frame()?;
    let target = context.create_drawable_render_target(120, 90);
    context.begin_render_to_target(&target);
    context.clear_background(Color::rgb(0.07, 0.10, 0.17));
    let source = Rect::new(frame.x as f32, frame.y as f32, frame.w as f32, frame.h as f32);
    let size = Vec2::new(frame.w as f32, frame.h as f32);
    context.set_default_camera();
    context.draw_texture_ex(&sheet, 14.0, 13.0, Color::WHITE, DrawTextureParams {
        source: Some(source), dest_size: Some(size), ..Default::default()
    });
    // Move the transformed copy relative to the reference through a real camera viewport.
    let camera = Camera2D {
        target: Vec2::new(50.0, 35.0),
        zoom: Vec2::new(2.0 / 100.0, 2.0 / 70.0),
        viewport: Some((10, 10, 100, 70)),
        ..Default::default()
    };
    context.set_camera(&camera);
    context.push_clip_rect(Rect::new(58.0, 18.0, 12.0, 28.0));
    context.draw_texture_ex(&sheet, 51.0, 13.0, Color::rgb(0.76, 0.98, 0.76), DrawTextureParams {
        source: Some(source), dest_size: Some(size), flip_x: true,
        rotation: 0.25, pivot: Some(size * 0.5), ..Default::default()
    });
    context.pop_clip_rect();
    context.set_default_camera();
    context.end_render_to_target();

    context.set_default_camera();
    context.clear_background(Color::BLACK);
    context.draw_render_target(&target, 0.0, 0.0, 240.0, 180.0);
    let caption = "Bishop WebGL2";
    let measured = context.measure_text(caption, 14.0);
    let drawn = context.draw_text(caption, 16.0, 130.0, 14.0, Color::WHITE);
    if measured.width <= 0.0 || (measured.width - drawn.width).abs() > 0.001 {
        return Err(format!("Demo proof text measurement disagrees with drawing: {measured:?} vs {drawn:?}"));
    }
    context.draw_text_ex("Demo Idle", 135.0, 150.0, TextParams {
        font_size: 12, font_scale_aspect: 1.15, rotation: 0.17,
        color: Color::SKYBLUE, ..Default::default()
    });
    context.end_frame()?;
    Ok((context, frame, measured.width))
}
