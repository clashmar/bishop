// editor/src/gui/prompts/world_edit_prompt.rs
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::*;
use bishop::prelude::*;
use engine_core::prelude::*;

/// Result an edit world prompt.
pub struct WorldEditResult {
    pub id: WorldId,
    pub name: Option<String>,
    pub sprite: Option<SpriteId>,
}

/// Prompt that draws:
///   * Edit name text field,
///   * World sprite picker,
///   * Confirm / Cancel buttons.
pub struct WorldEditPrompt {
    world_id: WorldId,
    name_id: WidgetId,
    rect: Rect,
    og_name: String,
    og_sprite: SpriteId,
    current_name: String,
    current_sprite: SpriteId,
}

impl WorldEditPrompt {
    /// Create a new prompt centred inside the supplied rect.
    pub fn new(
        world_id: WorldId,
        modal_rect: Rect,
        name_id: WidgetId,
        og_name: impl Into<String>,
        og_sprite: SpriteId,
    ) -> Self {
        let inner_w = modal_rect.w * 0.8;
        let inner_x = modal_rect.x + (modal_rect.w - inner_w) / 2.0;

        let total_h = modal_rect.h * 1.225;
        let inner_y = modal_rect.y + (total_h - modal_rect.h);

        let rect = Rect::new(inner_x, inner_y, inner_w, total_h);

        let name = og_name.into();

        Self {
            world_id,
            name_id,
            rect,
            og_name: name.clone(),
            og_sprite,
            current_name: name,
            current_sprite: og_sprite,
        }
    }

    /// Draws the widget and, return the result if confirmed/cancelled or None.
    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        asset_manager: &mut AssetManager,
    ) -> Option<WorldEditResult> {
        const GAP: f32 = 5.0;
        const FIELD_GAP: f32 = 30.0;

        let mut y = self.rect.y;

        // Name label
        let mut label_dims = ctx.draw_text(
            "Edit name:",
            self.rect.x,
            y,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );

        y += label_dims.height + GAP;

        // Name field
        let name_rect = Rect::new(self.rect.x, y, self.rect.w, FIELD_H);
        let (new_name, _) = TextInput::new(self.name_id, name_rect, &self.current_name)
            .max_len(33)
            .show(ctx);
        self.current_name = new_name;

        y += name_rect.h + FIELD_GAP;

        // Sprite label
        label_dims = ctx.draw_text(
            "Change sprite:",
            self.rect.x,
            y,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );

        y += label_dims.height + GAP;

        let sprite_rect = Rect::new(self.rect.x, y, self.rect.w, 30.0);
        if gui_sprite_picker(
            ctx,
            sprite_rect,
            &mut self.current_sprite,
            asset_manager,
            false,
        ) {
            // Widget updates the sprite
        }

        y += sprite_rect.h + FIELD_GAP;

        // Buttons
        let (confirm_rect, cancel_rect) = confirm_cancel_rects(self.rect, y);
        let confirm_clicked = Button::new(confirm_rect, "Confirm").show(ctx);
        let cancel_clicked = Button::new(cancel_rect, "Cancel").show(ctx);

        // Result
        if (confirm_clicked || Controls::enter(ctx)) && !self.current_name.trim().is_empty() {
            // Build a result that only contains the fields the user actually changed
            let name = if self.current_name != self.og_name {
                Some(self.current_name.clone())
            } else {
                None
            };
            let sprite = if self.current_sprite != self.og_sprite {
                Some(self.current_sprite)
            } else {
                None
            };
            return Some(WorldEditResult {
                id: self.world_id,
                name,
                sprite,
            });
        }

        if cancel_clicked || Controls::escape(ctx) {
            return Some(WorldEditResult {
                id: self.world_id,
                name: None,
                sprite: None,
            });
        }

        None
    }
}
