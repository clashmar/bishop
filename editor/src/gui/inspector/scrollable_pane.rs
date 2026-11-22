use crate::shared::scene_ui::inspector::{InspectorContent, InspectorContext, InspectorOutput};
use bishop::prelude::{Color, Rect, WgpuContext};
use bishop::Draw;
use engine_core::game::GameCtxMut;
use widgets::{is_mouse_over_dropdown_list, ScrollState, ScrollableArea, Widget};

const SCROLL_SPEED: f32 = 24.0;

/// Draws clipped pane modules and reveals any newly expanded module.
pub(super) fn draw<C: InspectorContent>(
    pane: &mut C,
    ctx: &mut WgpuContext,
    blocked: bool,
    game_ctx: &mut GameCtxMut,
    insp_ctx: &InspectorContext,
    body_rect: Rect,
    scroll_state: &mut ScrollState,
) -> InspectorOutput {
    ctx.draw_rectangle(
        body_rect.x,
        body_rect.y,
        body_rect.w,
        body_rect.h,
        Color::new(0., 0., 0., 0.6),
    );

    let total = pane.total_content_height(game_ctx, insp_ctx);
    let area = ScrollableArea::new(body_rect, total)
        .scroll_speed(SCROLL_SPEED)
        .blocked(is_mouse_over_dropdown_list(ctx))
        .begin(ctx, scroll_state);
    let content_rect = area.content_rect();
    let scrolled_content_rect = Rect::new(
        content_rect.x,
        content_rect.y + scroll_state.scroll_y,
        content_rect.w,
        content_rect.h,
    );

    ctx.push_clip_rect(body_rect);
    let mut output = pane.draw_modules(ctx, scrolled_content_rect, blocked, game_ctx, insp_ctx);
    ctx.pop_clip_rect();

    if let Some(expanded_rect) = output.expanded_module_rect.take() {
        let updated_total = pane.total_content_height(game_ctx, insp_ctx);
        let updated_scroll_range = (updated_total - body_rect.h).max(0.0);
        area.scroll_to_reveal(scroll_state, expanded_rect, updated_scroll_range);
    }

    area.draw_scrollbar(ctx, scroll_state);
    output
}
