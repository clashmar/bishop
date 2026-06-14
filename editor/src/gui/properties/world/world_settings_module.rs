use super::super::PropertyModule;
use crate::commands::game::EditWorldCmd;
use crate::editor_global::push_command;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::world::World;
use ::widgets::constants::{colors, layout};
use ::widgets::Checkbox;

/// Editable world-level settings (overlay flag, and future boolean properties).
pub struct WorldSettingsModule;

impl WorldSettingsModule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorldSettingsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldSettingsModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        world: &mut World,
        _game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let world_id = world.id;
        let mut overlay = world.overlay;

        let cb_rect = Rect::new(rect.x, rect.y + 6.0, layout::DEFAULT_CHECKBOX_DIMS, layout::DEFAULT_CHECKBOX_DIMS);
        if Checkbox::new(cb_rect, &mut overlay).show(ctx) {
            push_command(Box::new(EditWorldCmd::new(world_id, None, None).with_overlay(overlay)));
        }
        ctx.draw_text("Overlay", rect.x + layout::DEFAULT_CHECKBOX_DIMS + 6.0, rect.y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new().rows(1, layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "Settings"
    }
}
