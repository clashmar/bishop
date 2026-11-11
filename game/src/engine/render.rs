use crate::engine::*;
use bishop::prelude::*;
use engine_core::camera::CameraManager;
#[cfg(target_os = "emscripten")]
use engine_core::ecs::{Ecs, RoomCamera};
use engine_core::constants::window;
use engine_core::rendering::{RenderSystem, RoomRenderState, RoomVisibilityContext, render_room};
use engine_core::text::{collect_speech_bubbles, render_speech_bubbles};

fn gameplay_viewport_rect(screen_w: f32, screen_h: f32) -> Option<(i32, i32, i32, i32)> {
    if screen_w <= 0.0 || screen_h <= 0.0 {
        return None;
    }

    let aspect = window::DEFAULT_CAM_GRID_X / window::DEFAULT_CAM_GRID_Y;
    let window_aspect = screen_w / screen_h;

    let (width, height) = if window_aspect >= aspect {
        let height = screen_h.round() as i32;
        let width = (screen_h * aspect).round() as i32;
        (width, height)
    } else {
        let width = screen_w.round() as i32;
        let height = (screen_w / aspect).round() as i32;
        (width, height)
    };

    let x = ((screen_w.round() as i32 - width) / 2).max(0);
    let y = ((screen_h.round() as i32 - height) / 2).max(0);

    Some((x, y, width, height))
}

pub(super) fn gameplay_viewport(screen_w: f32, screen_h: f32) -> Rect {
    gameplay_viewport_rect(screen_w, screen_h)
        .map(|(x, y, w, h)| Rect::new(x as f32, y as f32, w as f32, h as f32))
        .unwrap_or(Rect::new(0.0, 0.0, screen_w, screen_h))
}

#[cfg(any(test, target_os = "emscripten"))]
fn browser_gameplay_viewport_rect(
    screen_w: f32,
    screen_h: f32,
    camera_zooms: impl Iterator<Item = Vec2>,
) -> Option<(i32, i32, i32, i32)> {
    let viewport = gameplay_viewport_rect(screen_w, screen_h)?;
    let reference = camera_zooms
        .filter(|zoom| zoom.x.is_finite() && zoom.y.is_finite() && zoom.x > 0.0 && zoom.y > 0.0)
        .map(|zoom| Vec2::new((2.0 / zoom.x).round(), (2.0 / zoom.y).round()))
        .reduce(Vec2::max)?;
    let (reference_w, reference_h) = (reference.x as i32, reference.y as i32);
    if reference_w <= 0 || reference_h <= 0 {
        return Some(viewport);
    }
    let scale = (viewport.2 / reference_w).min(viewport.3 / reference_h);
    if scale == 0 {
        return Some(viewport);
    }
    let (width, height) = (reference_w * scale, reference_h * scale);
    Some((
        viewport.0 + (viewport.2 - width) / 2,
        viewport.1 + (viewport.3 - height) / 2,
        width,
        height,
    ))
}

impl Engine {
    pub(crate) fn render_menus(&mut self, ctx: &PlatformContext) {
        if !self.menu_manager.has_active_menu() {
            return;
        }

        ctx.borrow_mut().flush_pending_draws();
        let viewport = gameplay_viewport(ctx.borrow().screen_width(), ctx.borrow().screen_height());
        self.menu_manager.set_viewport(viewport);
        let game_instance = self.game_instance.borrow();
        self.menu_manager
            .render(&mut *ctx.borrow_mut(), &game_instance.game.text_manager);
    }
}

/// Builds a camera for the current frame using interpolated position.
pub(super) fn build_render_camera(
    camera_manager: &CameraManager,
    alpha: f32,
    screen_w: f32,
    screen_h: f32,
) -> Camera2D {
    Camera2D {
        target: camera_manager.interpolated_target(alpha),
        zoom: camera_manager.active.camera.zoom,
        viewport: gameplay_viewport_rect(screen_w, screen_h),
        ..Default::default()
    }
}

#[cfg(target_os = "emscripten")]
pub(super) fn build_browser_render_camera(
    camera_manager: &CameraManager,
    alpha: f32,
    screen_w: f32,
    screen_h: f32,
    ecs: &Ecs,
) -> Camera2D {
    let mut camera = build_render_camera(camera_manager, alpha, screen_w, screen_h);
    camera.viewport = browser_gameplay_viewport_rect(
        screen_w,
        screen_h,
        ecs.get_store::<RoomCamera>().data.values().map(|room_camera| room_camera.zoom),
    ).or(camera.viewport);
    camera
}

/// Renders the game world for the current frame.
pub(super) fn render_scene<C: BishopContext>(
    ctx: &mut C,
    game_instance: &mut GameInstance,
    render_system: &mut RenderSystem,
    render_cam: &Camera2D,
    alpha: f32,
) {
    let render_start = std::time::Instant::now();
    let render_state = game_instance.current_render_state();
    let mut game_ctx = game_instance.game.ctx_mut();
    let prev_positions = &game_instance.prev_positions;

    render_room(
        ctx,
        &mut game_ctx,
        render_cam,
        render_state,
        alpha,
        Some(prev_positions),
    );

    render_system.render_time_ms = render_start.elapsed().as_secs_f32() * 1000.0;
    ctx.set_default_camera();
}

/// Renders all screen-space UI elements (speech bubbles, ui etc.).
pub fn render_screen_space<C: BishopContext>(
    ctx: &mut C,
    game_instance: &GameInstance,
    render_cam: &Camera2D,
    alpha: f32,
) {
    render_speech(
        ctx,
        game_instance,
        render_cam,
        game_instance.current_render_state(),
        alpha,
    );
}

/// Renders speech bubbles in screen space above the game world.
fn render_speech<C: BishopContext>(
    ctx: &mut C,
    game_instance: &GameInstance,
    render_cam: &Camera2D,
    render_state: RoomRenderState,
    alpha: f32,
) {
    let game_ctx = game_instance.game.ctx();
    let Some(current_room) = game_ctx.world.current_room() else {
        return;
    };
    let grid_size = game_ctx.world.grid_size;

    let room_ctx = RoomVisibilityContext {
        world: game_ctx.world,
        room: current_room,
        grid_size,
    };

    let bubbles = collect_speech_bubbles(
        game_ctx.ecs,
        game_ctx.sprite_manager,
        &room_ctx,
        render_state,
        alpha,
        Some(&game_instance.prev_positions),
    );

    render_speech_bubbles(
        ctx,
        &bubbles,
        &game_instance.game.text_manager.config,
        render_cam,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_uses_full_window_for_sixteen_nine_surface() {
        assert_eq!(
            gameplay_viewport_rect(1920.0, 1080.0),
            Some((0, 0, 1920, 1080))
        );
    }

    #[test]
    fn viewport_pillarboxes_wider_windows() {
        assert_eq!(
            gameplay_viewport_rect(2100.0, 1080.0),
            Some((90, 0, 1920, 1080))
        );
    }

    #[test]
    fn viewport_letterboxes_taller_windows() {
        assert_eq!(
            gameplay_viewport_rect(1600.0, 1200.0),
            Some((0, 150, 1600, 900))
        );
    }

    #[test]
    fn native_viewport_keeps_full_sixteen_nine_window() {
        assert_eq!(gameplay_viewport_rect(960.0, 540.0), Some((0, 0, 960, 540)));
    }

    #[test]
    fn browser_gameplay_viewport_integer_fits_the_widest_authored_camera() {
        let camera_zooms = [
            Vec2::new(2.0 / 128.0, 2.0 / 72.0),
            Vec2::new(2.0 / 256.0, 2.0 / 144.0),
        ];

        assert_eq!(
            browser_gameplay_viewport_rect(960.0, 540.0, camera_zooms.into_iter()),
            Some((96, 54, 768, 432))
        );
        assert_eq!(
            browser_gameplay_viewport_rect(1024.0, 576.0, camera_zooms.into_iter()),
            Some((0, 0, 1024, 576))
        );
    }

    #[test]
    fn browser_gameplay_viewport_falls_back_when_no_integer_scale_fits() {
        let zoom = Vec2::new(2.0 / 256.0, 2.0 / 144.0);

        assert_eq!(
            browser_gameplay_viewport_rect(192.0, 108.0, [zoom].into_iter()),
            Some((0, 0, 192, 108))
        );
    }
}
