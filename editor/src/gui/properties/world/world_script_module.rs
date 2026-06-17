use super::super::PropertyModule;
use crate::gui::widgets::script_module_core::ScriptModuleCore;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::world::World;

/// Assigns and edits a script on the world's singleton entity.
pub struct WorldScriptModule {
    core: ScriptModuleCore,
}

impl WorldScriptModule {
    /// Creates a new world script module.
    pub fn new() -> Self {
        Self {
            core: ScriptModuleCore::new(),
        }
    }
}

impl Default for WorldScriptModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldScriptModule {
    fn visible(&self, _world: &World, game_ctx: &GameCtxMut) -> bool {
        game_ctx
            .world
            .as_deref()
            .and_then(|w| w.singleton)
            .is_some()
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _world: &mut World,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let Some(entity) = game_ctx.world.as_deref().and_then(|w| w.singleton) else {
            return;
        };
        self.core.draw(ctx, rect, entity, game_ctx, false);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        self.core.body_layout()
    }

    fn title(&self) -> &str {
        "Script"
    }
}
