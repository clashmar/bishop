use crate::app::escape::modal_escape_requested;
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use widgets::constants::layout;

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
    sprite_picker_id: WidgetId,
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
        let total_h = PROMPT_TOP_PADDING
            + layout::DEFAULT_FONT_SIZE_16
            + PROMPT_TEXT_GAP
            + FIELD_H
            + PROMPT_SECTION_GAP
            + layout::DEFAULT_FONT_SIZE_16
            + PROMPT_TEXT_GAP
            + FIELD_H
            + PROMPT_SECTION_GAP
            + BUTTON_H
            + PROMPT_BOTTOM_PADDING;
        let rect = prompt_content_rect(modal_rect, total_h);

        let name = og_name.into();

        Self {
            world_id,
            name_id,
            sprite_picker_id: WidgetId::default(),
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
        asset_registry: &mut AssetRegistry,
        sprite_manager: &mut SpriteManager,
    ) -> Option<WorldEditResult> {
        let mut y = self.rect.y + PROMPT_TOP_PADDING;

        // Name label
        let mut label_dims = draw_prompt_label(ctx, "Edit name:", self.rect.x, y);

        y += label_dims.height + PROMPT_TEXT_GAP;

        // Name field
        let name_rect = Rect::new(self.rect.x, y, self.rect.w, FIELD_H);
        let (new_name, _) = TextInput::new(self.name_id, name_rect, &self.current_name)
            .max_len(33)
            .show(ctx);
        self.current_name = new_name;

        y += name_rect.h + PROMPT_SECTION_GAP;

        // Sprite label
        label_dims = draw_prompt_label(ctx, "Change sprite:", self.rect.x, y);

        y += label_dims.height + PROMPT_TEXT_GAP;

        let sprite_rect = Rect::new(self.rect.x, y, self.rect.w, 30.0);
        if gui_sprite_picker(
            ctx,
            sprite_rect,
            self.sprite_picker_id,
            &mut self.current_sprite,
            asset_registry,
            sprite_manager,
            false,
        ) {
            // Widget updates the sprite
        }

        y += sprite_rect.h + PROMPT_SECTION_GAP;

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

        if cancel_clicked || modal_escape_requested() {
            return Some(WorldEditResult {
                id: self.world_id,
                name: None,
                sprite: None,
            });
        }

        None
    }
}
