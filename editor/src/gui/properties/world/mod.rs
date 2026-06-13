pub mod world_script_module;
pub mod world_tags_module;

use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use engine_core::worlds::world::World;
use ::widgets::constants::layout;

use super::collapsible::CollapsiblePropertyModule;
use super::PropertyModule;
use crate::gui::gui_constants;
use crate::gui::text_input::{committed_name_change, draw_labeled_text_input};
use crate::shared::scene_ui::inspector::InspectorContent;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction, InspectorOutput};
use widgets::*;

/// Editable properties for the current world.
pub struct WorldProperties {
    input_id: WidgetId,
    pub modules: Vec<Box<dyn PropertyModule<World>>>,
}

impl WorldProperties {
    /// Creates a new world properties pane.
    pub fn new() -> Self {
        Self {
            input_id: WidgetId::default(),
            modules: vec![
                Box::new(CollapsiblePropertyModule::new(world_script_module::WorldScriptModule::new())),
                Box::new(CollapsiblePropertyModule::new(world_tags_module::WorldTagsModule::for_world())),
            ],
        }
    }
}

impl InspectorContent for WorldProperties {
    fn header_height(&self) -> f32 {
        gui_constants::inspector::HEADER_HEIGHT
    }

    fn draw_modules(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _blocked: bool,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        let mut output = InspectorOutput::default();
        let Some(world) = game_ctx.world.as_deref() else {
            return output;
        };
        let name = &world.name;

        let mut y = rect.y + 10.0;
        let content_w = rect.w - 20.0;

        let name_rect = Rect::new(rect.x + 10.0, y, content_w, 30.0);
        let (edited, commit) =
            draw_labeled_text_input(ctx, name_rect, "Name:", name, self.input_id);
        if let Some(new_name) = committed_name_change(name, &edited, commit) {
            output.host_action = Some(InspectorHostAction::RenameWorld(new_name));
        }
        y += 30.0 + layout::WIDGET_SPACING;

        // Collapsible modules
        let mut world_clone = world.clone();
        for module in &mut self.modules {
            if module.visible(&world_clone, game_ctx) {
                let h = module.height();
                let sub_rect = Rect::new(rect.x + 10.0, y, content_w, h);
                module.draw(ctx, sub_rect, &mut world_clone, game_ctx, _insp_ctx);
                y += h + layout::WIDGET_SPACING;
            }
        }

        // Write back changes from modules
        if let Some(w) = game_ctx.world.as_deref_mut() {
            w.tags = world_clone.tags;
        }

        output
    }

    fn total_content_height(
        &self,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> f32 {
        let Some(world) = game_ctx.world.as_deref() else {
            return 0.0;
        };

        let mut h = 30.0 + layout::WIDGET_SPACING; // name row

        for module in &self.modules {
            if module.visible(world, game_ctx) {
                h += module.height() + layout::WIDGET_SPACING;
            }
        }
        h + 20.0
    }
}
