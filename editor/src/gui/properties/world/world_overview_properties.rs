use super::super::collapsible::CollapsiblePropertyModule;
use super::super::PropertyModule;
use super::world_name_module::WorldNameModule;
use super::{collapse_world_modules, draw_world_modules, world_module_content_height};
use crate::gui::gui_constants;
use crate::shared::scene_ui::inspector::InspectorContent;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorOutput};
use bishop::{Rect, WgpuContext};
use engine_core::game::GameCtxMut;
use engine_core::worlds::world::World;

/// Overview-only properties for a selected world in the Game Editor.
pub struct WorldOverviewProperties {
    pub(super) name: WorldNameModule,
    pub(super) modules: Vec<Box<dyn PropertyModule<World>>>,
}

impl WorldOverviewProperties {
    /// Creates the Game Editor world overview pane.
    pub fn new() -> Self {
        Self {
            name: WorldNameModule::new(),
            modules: vec![Box::new(CollapsiblePropertyModule::new(
                super::world_visual_module::WorldVisualModule::new(),
            ))],
        }
    }

    /// Collapses the overview visual module.
    pub fn collapse_all(&mut self) {
        collapse_world_modules(&mut self.modules);
    }
}

impl Default for WorldOverviewProperties {
    fn default() -> Self {
        Self::new()
    }
}

impl InspectorContent for WorldOverviewProperties {
    fn header_height(&self) -> f32 {
        gui_constants::inspector::HEADER_HEIGHT
    }

    fn has_collapsible_modules(&self) -> bool {
        self.modules
            .iter()
            .any(|module| module.expansion_state().is_some())
    }

    fn collapse_all(&mut self) {
        WorldOverviewProperties::collapse_all(self);
    }

    fn draw_modules(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _blocked: bool,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        draw_world_modules(
            &mut self.name,
            &mut self.modules,
            ctx,
            rect,
            game_ctx,
            insp_ctx,
        )
    }

    fn total_content_height(
        &self,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> f32 {
        if game_ctx.world.is_none() {
            return 0.0;
        }
        world_module_content_height(&self.name, &self.modules, game_ctx) + 20.0
    }
}
