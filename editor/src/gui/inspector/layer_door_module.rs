use bishop::prelude::*;
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::generic_module::GenericModule;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::inspector::module::{CollapsibleComponentModule, InspectorModule};
use engine_core::ecs::*;
use engine_core::game::GameCtxMut;
use ::widgets::constants::layout;

const TITLE: &str = "Layer Door";
const WARNING_GAP: f32 = layout::WIDGET_SPACING;

#[derive(Default)]
pub struct LayerDoorModule {
    inner: GenericModule<LayerDoor>,
    show_warning: bool,
    warning_height: f32,
}

impl InspectorModule for LayerDoorModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        self.inner.undo_component_type()
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        self.inner.visible(ecs, entity)
    }

    fn removable(&self) -> bool {
        self.inner.removable()
    }

    fn was_input_active(&self) -> bool {
        self.inner.was_input_active()
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let mut layout = self.inner.body_layout();
        if self.show_warning {
            layout = layout.gap(WARNING_GAP).block(self.warning_height);
        }
        layout
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        self.inner.draw(ctx, blocked, rect, game_ctx, entity);
        self.show_warning = false;
        self.warning_height = 0.0;

        let Some(world) = game_ctx.world.as_deref() else {
            return;
        };

        let Err(issue) = validate_layer_door(&game_ctx.ecs, world, entity) else {
            return;
        };

        self.show_warning = true;
        self.warning_height = ctx.draw_text_wrapped(
            issue.message(),
            rect.x,
            rect.y + self.inner.body_layout().height() + WARNING_GAP,
            layout::FIELD_TEXT_SIZE_16 - 2.0,
            Color::GOLD,
            rect.w,
        );
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: LayerDoor::TYPE_NAME,
        title: TITLE,
        factory: || Box::new(
            CollapsibleComponentModule::new(
                crate::gui::inspector::layer_door_module::LayerDoorModule::default()
            ).with_title(TITLE)
        ),
        allowed_for: Some(|entity, ecs| {
            ecs.has::<engine_core::ecs::CurrentRoom>(entity)
                && !ecs.has::<engine_core::ecs::Player>(entity)
                && !ecs.has::<engine_core::ecs::PlayerProxy>(entity)
        }),
    }
}
