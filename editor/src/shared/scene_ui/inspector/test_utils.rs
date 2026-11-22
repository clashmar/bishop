use bishop::prelude::{Rect, WgpuContext};
use engine_core::ecs::ecs::Ecs;
use engine_core::ecs::entity::Entity;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::inspector::module::InspectorModule;
use engine_core::game::GameCtxMut;

/// A test inspector module with configurable collapsible state.
pub(crate) struct FakeInspectorModule {
    expanded: Option<bool>,
}

impl FakeInspectorModule {
    /// Creates a fake module with the supplied expansion state.
    pub(crate) fn new(expanded: Option<bool>) -> Self {
        Self { expanded }
    }
}

impl InspectorModule for FakeInspectorModule {
    fn visible(&self, _ecs: &Ecs, _entity: Entity) -> bool {
        true
    }

    fn draw(
        &mut self,
        _ctx: &mut WgpuContext,
        _blocked: bool,
        _rect: Rect,
        _game_ctx: &mut GameCtxMut,
        _entity: Entity,
    ) {
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
    }

    fn undo_component_type(&self) -> Option<&'static str> {
        None
    }

    fn expansion_state(&self) -> Option<bool> {
        self.expanded
    }

    fn set_expanded(&mut self, expanded: bool) {
        if self.expanded.is_some() {
            self.expanded = Some(expanded);
        }
    }
}
