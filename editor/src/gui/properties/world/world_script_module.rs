use super::super::PropertyModule;
use crate::gui::widgets::script_picker_row::draw_script_picker_row;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::world::World;
use widgets::*;
use ::widgets::constants::layout;

/// Assigns a script to the world's singleton entity.
pub struct WorldScriptModule {
    picker_id: WidgetId,
}

impl WorldScriptModule {
    pub fn new() -> Self {
        Self {
            picker_id: WidgetId::default(),
        }
    }
}

impl Default for WorldScriptModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldScriptModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _world: &mut World,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let Some(entity) = game_ctx
            .world
            .as_deref()
            .and_then(|w| w.singleton)
        else {
            return;
        };

        let row_rect = Rect::new(
            rect.x,
            rect.y + layout::WIDGET_SPACING,
            rect.w,
            layout::DEFAULT_FIELD_HEIGHT,
        );

        draw_script_picker_row(
            ctx,
            row_rect,
            self.picker_id,
            entity,
            game_ctx.ecs,
            game_ctx.asset_registry,
            game_ctx.script_manager,
            false,
        );
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new().rows(1, layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "Script"
    }
}
