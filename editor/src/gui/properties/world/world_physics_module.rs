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
use ::widgets::{NumberInput, WidgetId};

const ROW_H: f32 = 30.0;
const LABEL_Y_OFFSET: f32 = 20.0;
const COLON_GAP: f32 = 8.0;
const ROW_LABEL_INPUT_GAP: f32 = 10.0;

/// Editable world-level physics settings.
pub struct WorldPhysicsModule {
    gravity_id: WidgetId,
}

impl WorldPhysicsModule {
    /// Creates a world physics module.
    pub fn new() -> Self {
        Self {
            gravity_id: WidgetId::default(),
        }
    }
}

impl Default for WorldPhysicsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldPhysicsModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        world: &mut World,
        _game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let world_id = world.id;
        let row_y = rect.y + layout::WIDGET_SPACING;
        let label_w = measure_text(ctx, "Gravity:", layout::FIELD_TEXT_SIZE_16).width + COLON_GAP;
        ctx.draw_text(
            "Gravity:",
            rect.x + layout::WIDGET_PADDING,
            row_y + LABEL_Y_OFFSET,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

        let input_x = rect.x + layout::WIDGET_PADDING + label_w + ROW_LABEL_INPUT_GAP;
        let input_w = rect.x + rect.w - layout::WIDGET_PADDING - input_x;
        let gravity_rect = Rect::new(input_x, row_y, input_w.max(64.0), ROW_H);
        let gravity = world.physics.gravity;
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
            .rows(1, layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "Physics"
    }
}
