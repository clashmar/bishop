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

const ROW_H: f32 = 30.0;
const LABEL_Y_OFFSET: f32 = 20.0;
const CHECKBOX_LABEL_GAP: f32 = 6.0;

/// Editable general world settings.
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
        let y = rect.y;

        let mut overlay = world.overlay;
        let cb_rect = Rect::new(
            rect.x + layout::WIDGET_PADDING,
            y + (ROW_H - layout::DEFAULT_CHECKBOX_DIMS) / 2.0,
            layout::DEFAULT_CHECKBOX_DIMS,
            layout::DEFAULT_CHECKBOX_DIMS,
        );
        if Checkbox::new(cb_rect, &mut overlay).show(ctx) {
            push_command(Box::new(EditWorldCmd::new(world_id, None, None).with_overlay(overlay)));
        }
        ctx.draw_text(
            "Overlay",
            rect.x + layout::WIDGET_PADDING + layout::DEFAULT_CHECKBOX_DIMS + CHECKBOX_LABEL_GAP,
            y + LABEL_Y_OFFSET,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .top_padding(layout::WIDGET_SPACING)
            .rows(1, layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "General Settings"
    }
}
