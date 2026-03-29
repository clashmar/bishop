// game/src/engine/render.rs
use crate::engine::*;
use bishop::prelude::*;
use engine_core::prelude::*;

impl Engine {
    pub(crate) fn render_menus(&mut self, ctx: &PlatformContext) {
        if !self.menu_manager.has_active_menu() {
            return;
        }

        ctx.borrow_mut().flush_if_needed();
        let viewport = self.render_system.viewport_rect(&*ctx.borrow());
        self.menu_manager.set_viewport(viewport);
        let game_instance = self.game_instance.borrow();
        self.menu_manager
            .render(&mut *ctx.borrow_mut(), &game_instance.game.text_manager);
    }
}

/// Builds a camera for the current frame using interpolated position.
pub(super) fn build_render_camera(camera_manager: &CameraManager, alpha: f32) -> Camera2D {
    Camera2D {
        target: camera_manager.interpolated_target(alpha),
        zoom: camera_manager.active.camera.zoom,
        ..Default::default()
    }
}

/// Renders the game world for the current frame.
pub(super) fn render_scene<C: BishopContext>(
    ctx: &mut C,
    game_instance: &mut GameInstance,
    render_system: &mut RenderSystem,
    render_cam: &Camera2D,
    alpha: f32,
) {
    let mut game_ctx = game_instance.game.ctx_mut();
    let prev_positions = &game_instance.prev_positions;

    render_system.resize_for_camera(render_cam.zoom);
    render_system.begin_scene(ctx);

    render_room(
        ctx,
        &mut game_ctx,
        render_system,
        render_cam,
        alpha,
        Some(prev_positions),
    );

    render_system.end_scene(ctx);
    render_system.present_game(ctx);
}

/// Renders all screen-space UI elements (speech bubbles, ui etc.).
pub fn render_screen_space<C: BishopContext>(
    ctx: &mut C,
    game_instance: &GameInstance,
    render_cam: &Camera2D,
    alpha: f32,
) {
    render_speech(ctx, game_instance, render_cam, alpha);
}

/// Renders speech bubbles in screen space above the game world.
fn render_speech<C: BishopContext>(
    ctx: &mut C,
    game_instance: &GameInstance,
    render_cam: &Camera2D,
    alpha: f32,
) {
    let game_ctx = game_instance.game.ctx();
    let Some(current_room) = game_ctx.cur_world.current_room() else {
        return;
    };
    let grid_size = game_ctx.cur_world.grid_size;

    let bubbles = collect_speech_bubbles(
        game_ctx.ecs,
        game_ctx.asset_manager,
        current_room.id,
        alpha,
        Some(&game_instance.prev_positions),
        grid_size,
    );

    render_speech_bubbles(
        ctx,
        &bubbles,
        &game_instance.game.text_manager.config,
        render_cam,
        grid_size,
    );
}
