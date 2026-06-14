use crate::gui::widgets::script_module_core::ScriptModuleCore;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::*;
use engine_core::game::GameCtxMut;

/// Inspector module for the Script component.
#[derive(Default)]
pub struct ScriptModule {
    core: ScriptModuleCore,
}

impl InspectorModule for ScriptModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        ScriptModuleCore::undo_component_type()
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<Script>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        Ecs::remove_component::<Script>(game_ctx, entity);
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        self.core.draw(ctx, rect, entity, game_ctx, blocked);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        self.core.body_layout()
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: <engine_core::ecs::Script>::TYPE_NAME,
        title: <engine_core::ecs::Script>::TYPE_NAME,
        factory: || {
            Box::new(
                CollapsibleComponentModule::new(
                    crate::gui::inspector::script_module::ScriptModule::default()
                )
                .with_title(<engine_core::ecs::Script>::TYPE_NAME)
            )
        },
        allowed_for: None,
    }
}
