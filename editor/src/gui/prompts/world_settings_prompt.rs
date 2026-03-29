// editor/src/gui/prompts/world_settings_prompt.rs
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::*;
use bishop::prelude::*;
use engine_core::prelude::*;

/// Result of a world settings prompt.
pub struct WorldSettingsResult {
    pub id: WorldId,
    pub grid_size: Option<f32>,
}

/// Prompt that draws:
///   * Grid size number input,
///   * Confirm / Cancel buttons.
pub struct WorldSettingsPrompt {
    world_id: WorldId,
    grid_size_id: WidgetId,
    rect: Rect,
    og_grid_size: f32,
    current_grid_size: f32,
}

impl WorldSettingsPrompt {
    /// Create a new prompt centred inside the supplied rect.
    pub fn new(
        world_id: WorldId,
        modal_rect: Rect,
        grid_size_id: WidgetId,
        og_grid_size: f32,
    ) -> Self {
        let inner_w = modal_rect.w * 0.8;
        let inner_x = modal_rect.x + (modal_rect.w - inner_w) / 2.0;

        let total_h = modal_rect.h * 1.225;
        let inner_y = modal_rect.y + (total_h - modal_rect.h);

        let rect = Rect::new(inner_x, inner_y, inner_w, total_h);

        Self {
            world_id,
            grid_size_id,
            rect,
            og_grid_size,
            current_grid_size: og_grid_size,
        }
    }

    /// Draws the widget and, return the result if confirmed/cancelled or None.
    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<WorldSettingsResult> {
        const GAP: f32 = 5.0;
        const FIELD_GAP: f32 = 30.0;

        let mut y = self.rect.y;

        // Grid size label
        let label_dims = ctx.draw_text(
            "Grid Size:",
            self.rect.x,
            y,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );

        y += label_dims.height + GAP;

        // Grid size field
        let grid_size_rect = Rect::new(self.rect.x, y, self.rect.w, FIELD_H);
        let new_grid_size =
            NumberInput::new(self.grid_size_id, grid_size_rect, self.current_grid_size)
                .min(8.0)
                .max(64.0)
                .show(ctx);
        self.current_grid_size = new_grid_size;

        y += grid_size_rect.h + FIELD_GAP;

        // Buttons
        let (confirm_rect, cancel_rect) = confirm_cancel_rects(self.rect, y);
        let confirm_clicked = Button::new(confirm_rect, "Confirm").show(ctx);
        let cancel_clicked = Button::new(cancel_rect, "Cancel").show(ctx);

        // Result
        if confirm_clicked || Controls::enter(ctx) {
            let grid_size = if self.current_grid_size != self.og_grid_size {
                Some(self.current_grid_size)
            } else {
                None
            };
            return Some(WorldSettingsResult {
                id: self.world_id,
                grid_size,
            });
        }

        if cancel_clicked || Controls::escape(ctx) {
            return Some(WorldSettingsResult {
                id: self.world_id,
                grid_size: None,
            });
        }

        None
    }
}
