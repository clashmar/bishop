// editor/src/main.rs

// Tells windows if it's a console app or not (console is useful in debug)
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::app::Editor;
use crate::editor_assets::assets::*;
use crate::editor_global::*;
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::prelude::*;

mod app;
mod canvas;
mod commands;
mod editor_assets;
mod editor_global;
mod game;
mod gui;
mod menu;
mod playtest;
mod room;
mod shared;
mod storage;
mod tilemap;
mod world;

/// Wrapper struct for running the editor via BishopApp.
struct EditorApp {
    current_window_size: (u32, u32),
}

impl EditorApp {
    fn new() -> Self {
        Self {
            current_window_size: (0, 0),
        }
    }
}

impl BishopApp for EditorApp {
    async fn init(&mut self, ctx: PlatformContext) {
        onscreen_info!("Starting editor.");

        // Initialize logging
        init_file_logger();

        if !ensure_save_root() {
            // User cancelled
            onscreen_warn!("No save root selected. Exiting.");
            std::process::exit(0);
        }

        let games_path = absolute_save_root();
        if let Err(e) = std::fs::create_dir_all(&games_path) {
            onscreen_warn!("Failed to create save root: {}", e);
            std::process::exit(1);
        }

        match Editor::new(ctx.clone()).await {
            Ok(editor) => {
                // This allows global access to services
                set_editor(editor);
            }
            Err(e) => {
                onscreen_warn!("Failed to initialize editor: {}", e);
                std::process::exit(1);
            }
        }
    }

    fn on_exit(&mut self) {
        with_editor(|editor| {
            editor.game.asset_manager.flush_pending_removals();
            editor.game.script_manager.flush_pending_removals();
            editor.save();
        });
    }

    async fn frame(&mut self, ctx: PlatformContext) {
        let mut ctx_ref = ctx.borrow_mut();
        let cur_screen = (
            ctx_ref.screen_width() as u32,
            ctx_ref.screen_height() as u32,
        );
        if cur_screen != self.current_window_size {
            with_editor(|editor| editor.render_system.resize(cur_screen.0, cur_screen.1));
            self.current_window_size = cur_screen;
        }

        widgets_frame_start(&mut *ctx_ref);

        with_editor(|editor| {
            editor.update(&mut ctx_ref);
            editor.draw(&mut ctx_ref);
        });

        widgets_frame_end(&mut *ctx_ref);

        apply_pending_commands();
    }
}

fn main() -> Result<(), RunError> {
    let window_width = FIXED_WINDOW_WIDTH.clamp(MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH);
    let window_height = FIXED_WINDOW_HEIGHT.clamp(MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT);

    let icon = WindowIcon::Rgba {
        small: Some(IconData::new(ICON_SMALL.to_vec(), 16, 16)),
        medium: Some(IconData::new(ICON_MEDIUM.to_vec(), 32, 32)),
        large: Some(IconData::new(ICON_BIG.to_vec(), 64, 64)),
    };

    let config = WindowConfig::new("Bishop Engine")
        .with_size(window_width as u32, window_height as u32)
        .with_resizable(true)
        .with_icon(icon);

    let app = EditorApp::new();
    run_backend(config, app)
}
