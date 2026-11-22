use crate::app::EditorMode;
use crate::gui::gui_constants::{inspector, INSET};
use crate::gui::menu_bar::menu_button;
use crate::gui::widgets::menu_widgets::menu_button_with_font_size;
use bishop::prelude::{Rect, WgpuContext};
use bishop::Draw;
use widgets::constants::layout;
use widgets::with_theme;

const CYCLE_ARROW_FONT_SIZE: f32 = layout::DEFAULT_FONT_SIZE_16;

/// Button geometry and availability for the inspector action strip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ActionStripLayout {
    pub(super) strip_rect: Rect,
    pub(super) collapse_rect: Rect,
    pub(super) collapse_disabled: bool,
    pub(super) cycle_previous_rect: Option<Rect>,
    pub(super) cycle_next_rect: Option<Rect>,
}

/// Action requested by a click in the inspector strip.
pub(super) enum ActionStripAction {
    CollapseAll,
    CyclePrevious,
    CycleNext,
}

/// Computes strip geometry from the current inspector pane and mode.
pub(super) fn layout(
    content_frame_rect: Rect,
    entity_pane: bool,
    command_mode: EditorMode,
    cycle_entity_count: usize,
    has_collapsible_modules: bool,
) -> ActionStripLayout {
    let strip_rect = Rect::new(
        content_frame_rect.x,
        content_frame_rect.y,
        content_frame_rect.w,
        inspector::ACTION_STRIP_HEIGHT,
    );
    let button_y = strip_rect.y + (strip_rect.h - inspector::ACTION_BUTTON_SIZE) / 2.0;
    let collapse_rect = Rect::new(
        strip_rect.x + strip_rect.w - INSET - inspector::ACTION_BUTTON_SIZE,
        button_y,
        inspector::ACTION_BUTTON_SIZE,
        inspector::ACTION_BUTTON_SIZE,
    );
    let cycle_visible = entity_pane
        && matches!(command_mode, EditorMode::Room(_))
        && cycle_entity_count > 1;
    let cycle_next_rect = cycle_visible.then(|| {
        Rect::new(
            collapse_rect.x - inspector::ACTION_BUTTON_GAP - inspector::ACTION_BUTTON_SIZE,
            button_y,
            inspector::ACTION_BUTTON_SIZE,
            inspector::ACTION_BUTTON_SIZE,
        )
    });
    let cycle_previous_rect = cycle_next_rect.map(|next_rect| {
        Rect::new(
            next_rect.x - inspector::ACTION_BUTTON_GAP - inspector::ACTION_BUTTON_SIZE,
            button_y,
            inspector::ACTION_BUTTON_SIZE,
            inspector::ACTION_BUTTON_SIZE,
        )
    });

    ActionStripLayout {
        strip_rect,
        collapse_rect,
        collapse_disabled: !has_collapsible_modules,
        cycle_previous_rect,
        cycle_next_rect,
    }
}

impl ActionStripLayout {
    /// Returns every strip button rectangle for inspector hit testing.
    pub(super) fn interactive_rects(&self) -> Vec<Rect> {
        let mut rects = vec![self.collapse_rect];
        if let Some(previous) = self.cycle_previous_rect {
            rects.push(previous);
        }
        if let Some(next) = self.cycle_next_rect {
            rects.push(next);
        }
        rects
    }
}

/// Draws the strip and returns the action requested by the user, if any.
pub(super) fn draw(
    ctx: &mut WgpuContext,
    layout: ActionStripLayout,
    blocked: bool,
) -> Option<ActionStripAction> {
    ctx.draw_rectangle(
        layout.strip_rect.x,
        layout.strip_rect.y,
        layout.strip_rect.w,
        layout.strip_rect.h,
        with_theme(|theme| theme.panel),
    );
    let collapse_clicked = menu_button(
        ctx,
        layout.collapse_rect,
        "-",
        false,
        blocked || layout.collapse_disabled,
    );
    let previous_clicked = layout.cycle_previous_rect.is_some_and(|rect| {
        menu_button_with_font_size(ctx, rect, "<", false, blocked, CYCLE_ARROW_FONT_SIZE)
    });
    let next_clicked = layout.cycle_next_rect.is_some_and(|rect| {
        menu_button_with_font_size(ctx, rect, ">", false, blocked, CYCLE_ARROW_FONT_SIZE)
    });

    if collapse_clicked && !layout.collapse_disabled {
        Some(ActionStripAction::CollapseAll)
    } else if previous_clicked {
        Some(ActionStripAction::CyclePrevious)
    } else if next_clicked {
        Some(ActionStripAction::CycleNext)
    } else {
        None
    }
}
