use super::super::PropertyModule;
use crate::commands::game::EditWorldCmd;
use crate::editor_global::push_command;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::ui::measure_text;
use engine_core::worlds::world::World;
use ::widgets::constants::{colors, layout};
use ::widgets::{Checkbox, NumberInput, WidgetId};

const ROW_H: f32 = 30.0;
const LABEL_Y_OFFSET: f32 = 20.0;
const COLON_GAP: f32 = 8.0;
const ROW_LABEL_INPUT_GAP: f32 = 10.0;
const CHECKBOX_LABEL_GAP: f32 = 6.0;

/// Editable world-level settings (overlay flag, gravity, and future properties).
pub struct WorldSettingsModule {
    gravity_id: WidgetId,
}

impl WorldSettingsModule {
    pub fn new() -> Self {
        Self {
            gravity_id: WidgetId::default(),
        }
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
        let mut y = rect.y;

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

        y += ROW_H + layout::WIDGET_SPACING;

        let label_w = measure_text(ctx, "Gravity:", layout::FIELD_TEXT_SIZE_16).width + COLON_GAP;
        ctx.draw_text(
            "Gravity:",
            rect.x + layout::WIDGET_PADDING,
            y + LABEL_Y_OFFSET,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

        let input_x = rect.x + layout::WIDGET_PADDING + label_w + ROW_LABEL_INPUT_GAP;
        let input_w = rect.x + rect.w - layout::WIDGET_PADDING - input_x;
        let gravity_rect = Rect::new(input_x, y, input_w.max(64.0), ROW_H);
        let gravity = world.gravity;
        let (new_gravity, _) = NumberInput::new(self.gravity_id, gravity_rect, gravity)
            .min(0.0)
            .show(ctx);
        if (new_gravity - gravity).abs() > f32::EPSILON {
            push_command(Box::new(EditWorldCmd::new(world_id, None, None).with_gravity(new_gravity)));
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .top_padding(layout::WIDGET_SPACING)
            .rows(2, layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "Settings"
    }
}
